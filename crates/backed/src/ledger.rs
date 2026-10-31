//! Per-token books with the core invariant `owed_total <= shadow_held`.
//!
//! - `owed_total`: what the contract owes everyone (sum of balances).
//! - `shadow_held`: tokens the contract knows it holds. It moves only with
//!   verified inbound flows and with outbound transfers this ledger started.
//! - `out_started`: a counter that only goes up, used by race-free reconcile.
//!
//! Order of operations is what keeps async timing safe: debit *before*
//! sending tokens, credit *after* receiving them. Then any timing gap only
//! makes real holdings look higher than the books, never lower.

use near_sdk::json_types::U128;
use near_sdk::store::LookupMap;
use near_sdk::{env, near, AccountId};

use crate::amount::{Claimed, Delivered};
use crate::refund::cap_refund;
use crate::ERR_OWED_EXCEEDS_HELD;

#[near(serializers = [borsh, json])]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BookStatus {
    Healthy,
    /// Real holdings were found below the books. Withdrawals of this token
    /// are refused until a later reconcile finds the books backed again.
    Breach { since_block: u64, shortfall: U128 },
}

#[near(serializers = [borsh])]
#[derive(Clone, Debug, PartialEq, Eq)]
struct TokenBook {
    owed_total: u128,
    shadow_held: u128,
    out_started: u128,
    status: BookStatus,
}

impl TokenBook {
    fn new() -> Self {
        Self { owed_total: 0, shadow_held: 0, out_started: 0, status: BookStatus::Healthy }
    }
}

/// Standard solvency view (draft NEP): `backed_solvency(token)`.
#[near(serializers = [json])]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Solvency {
    pub owed: U128,
    pub shadow_held: U128,
    pub status: BookStatus,
}

/// A withdrawal that has been debited but whose transfer hasn't resolved yet.
/// Pass it to the private callback and call [`Ledger::settle_withdraw`].
#[near(serializers = [borsh, json])]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingOut {
    pub token: AccountId,
    pub owner: AccountId,
    pub amount: U128,
}

/// Snapshot taken when a reconcile starts.
#[near(serializers = [borsh, json])]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReconcileTicket {
    pub token: AccountId,
    pub shadow_held_at_start: U128,
    pub out_started_at_start: U128,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReconcileOutcome {
    Healthy,
    Breach { shortfall: u128 },
}

#[near(serializers = [borsh])]
pub struct Ledger {
    books: LookupMap<AccountId, TokenBook>,
    balances: LookupMap<(AccountId, AccountId), u128>,
}

impl Ledger {
    /// `prefix` must be unique within the contract's storage.
    pub fn new(prefix: &[u8]) -> Self {
        let mut books_key = prefix.to_vec();
        books_key.push(b'b');
        let mut balances_key = prefix.to_vec();
        balances_key.push(b'l');
        Self { books: LookupMap::new(books_key), balances: LookupMap::new(balances_key) }
    }

    pub fn register_token(&mut self, token: AccountId) {
        if !self.books.contains_key(&token) {
            self.books.insert(token, TokenBook::new());
        }
    }

    pub fn is_registered(&self, token: &AccountId) -> bool {
        self.books.contains_key(token)
    }

    pub fn balance_of(&self, token: &AccountId, owner: &AccountId) -> u128 {
        self.balances.get(&(token.clone(), owner.clone())).copied().unwrap_or(0)
    }

    pub fn solvency(&self, token: &AccountId) -> Option<Solvency> {
        self.books.get(token).map(|b| Solvency {
            owed: U128(b.owed_total),
            shadow_held: U128(b.shadow_held),
            status: b.status.clone(),
        })
    }

    /// Credits verified tokens that arrived at this contract.
    pub fn credit(&mut self, token: &AccountId, owner: &AccountId, amount: Delivered) {
        let x = amount.amount();
        let book = self.book_mut(token);
        book.shadow_held = book.shadow_held.checked_add(x).expect("held overflow");
        book.owed_total = book.owed_total.checked_add(x).expect("owed overflow");
        let book = book.clone();
        assert_backed(&book);
        self.books.insert(token.clone(), book);
        self.add_balance(token, owner, x);
    }

    /// Credits tokens that arrived with a follow-up call (forwarding, notifying
    /// another contract). Call [`Ledger::finalize_followup`] in the resolve.
    pub fn credit_with_followup(&mut self, token: &AccountId, owner: &AccountId, amount: Delivered) -> PendingIn {
        let x = amount.amount();
        self.credit(token, owner, amount);
        PendingIn { token: token.clone(), owner: owner.clone(), deposited: U128(x) }
    }

    /// Settles a follow-up: refunds at most what was deposited and what the
    /// owner still has. Never panics, so the resolve callback can't fail
    /// (NEAR Intents, Oct 2026: a failing resolve left unbacked balances).
    /// Returns the refund to report back to the token contract as "unused".
    pub fn finalize_followup(&mut self, pending: &PendingIn, requested_refund: u128) -> u128 {
        let Some(book) = self.books.get(&pending.token).cloned() else { return 0 };
        let balance = self.balance_of(&pending.token, &pending.owner);
        let refund = cap_refund(requested_refund, pending.deposited.0, balance)
            .min(book.owed_total)
            .min(book.shadow_held);
        if refund == 0 {
            return 0;
        }
        let mut book = book;
        book.owed_total -= refund;
        book.shadow_held -= refund;
        self.books.insert(pending.token.clone(), book);
        self.balances.insert((pending.token.clone(), pending.owner.clone()), balance - refund);
        refund
    }

    /// Moves a balance between two accounts inside the contract.
    pub fn transfer_internal(&mut self, token: &AccountId, from: &AccountId, to: &AccountId, amount: Claimed) {
        let x = amount.0;
        self.sub_balance(token, from, x);
        self.add_balance(token, to, x);
    }

    /// Debits before sending tokens out. Refused while the token is in breach.
    pub fn debit_for_withdraw(&mut self, token: &AccountId, owner: &AccountId, amount: Claimed) -> PendingOut {
        let x = amount.0;
        let mut book = self.book_mut(token).clone();
        assert!(matches!(book.status, BookStatus::Healthy), "BACKED: withdrawals paused, token in breach");
        self.sub_balance(token, owner, x);
        book.owed_total = book.owed_total.checked_sub(x).expect("owed underflow");
        book.shadow_held = book.shadow_held.checked_sub(x).expect("held underflow");
        book.out_started = book.out_started.checked_add(x).expect("counter overflow");
        assert_backed(&book);
        self.books.insert(token.clone(), book);
        PendingOut { token: token.clone(), owner: owner.clone(), amount: U128(x) }
    }

    /// Settles a withdrawal in the private callback. On failure the tokens are
    /// back with the contract, so the owner is re-credited.
    pub fn settle_withdraw(&mut self, pending: &PendingOut, transfer_succeeded: bool) {
        if transfer_succeeded {
            return;
        }
        let x = pending.amount.0;
        let mut book = self.book_mut(&pending.token).clone();
        book.owed_total = book.owed_total.saturating_add(x);
        book.shadow_held = book.shadow_held.saturating_add(x);
        self.books.insert(pending.token.clone(), book);
        self.add_balance(&pending.token, &pending.owner, x);
    }

    /// Step 1 of reconcile: snapshot before asking the token contract.
    pub fn begin_reconcile(&self, token: &AccountId) -> ReconcileTicket {
        let book = self.books.get(token).expect("BACKED: unknown token");
        ReconcileTicket {
            token: token.clone(),
            shadow_held_at_start: U128(book.shadow_held),
            out_started_at_start: U128(book.out_started),
        }
    }

    /// Step 2 of reconcile, in the callback with the token's real balance.
    ///
    /// Check: `real + (out_started_now - out_started_at_start) >= shadow_held_at_start`.
    /// Deposits during the gap only raise `real`; transfers started during
    /// the gap are added back by the counter. So it fails only when tokens left
    /// without this ledger knowing (a failed resolve, a fee/burn token, a bug).
    pub fn finish_reconcile(&mut self, ticket: &ReconcileTicket, real_balance: u128) -> ReconcileOutcome {
        let mut book = self.book_mut(&ticket.token).clone();
        let started_since = book.out_started.saturating_sub(ticket.out_started_at_start.0);
        let explained = real_balance.saturating_add(started_since);
        let outcome = if explained >= ticket.shadow_held_at_start.0 {
            book.status = BookStatus::Healthy;
            ReconcileOutcome::Healthy
        } else {
            let shortfall = ticket.shadow_held_at_start.0 - explained;
            if matches!(book.status, BookStatus::Healthy) {
                book.status = BookStatus::Breach { since_block: env::block_height(), shortfall: U128(shortfall) };
                env::log_str(&format!(
                    "EVENT_JSON:{{\"standard\":\"backed\",\"version\":\"0.1.0\",\"event\":\"backing_breach\",\"data\":[{{\"token\":\"{}\",\"shortfall\":\"{}\"}}]}}",
                    ticket.token, shortfall
                ));
            }
            ReconcileOutcome::Breach { shortfall }
        };
        self.books.insert(ticket.token.clone(), book);
        outcome
    }

    fn book_mut(&mut self, token: &AccountId) -> &mut TokenBook {
        self.books.get_mut(token).expect("BACKED: unknown token")
    }

    fn add_balance(&mut self, token: &AccountId, owner: &AccountId, x: u128) {
        let key = (token.clone(), owner.clone());
        let b = self.balances.get(&key).copied().unwrap_or(0);
        self.balances.insert(key, b.checked_add(x).expect("balance overflow"));
    }

    fn sub_balance(&mut self, token: &AccountId, owner: &AccountId, x: u128) {
        let key = (token.clone(), owner.clone());
        let b = self.balances.get(&key).copied().unwrap_or(0);
        assert!(b >= x, "BACKED: insufficient balance");
        self.balances.insert(key, b - x);
    }
}

/// A deposit with a follow-up call still pending. Pass to the resolve callback.
#[near(serializers = [borsh, json])]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingIn {
    pub token: AccountId,
    pub owner: AccountId,
    pub deposited: U128,
}

fn assert_backed(book: &TokenBook) {
    assert!(book.owed_total <= book.shadow_held, "{}", ERR_OWED_EXCEEDS_HELD);
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::VMContextBuilder;
    use near_sdk::testing_env;

    fn acct(s: &str) -> AccountId {
        s.parse().unwrap()
    }

    fn setup() -> (Ledger, AccountId) {
        testing_env!(VMContextBuilder::new().build());
        let mut l = Ledger::new(b"t");
        let t = acct("usdc.near");
        l.register_token(t.clone());
        (l, t)
    }

    #[test]
    fn deposit_withdraw_roundtrip() {
        let (mut l, t) = setup();
        let alice = acct("alice.near");
        l.credit(&t, &alice, Delivered::for_tests(100));
        let p = l.debit_for_withdraw(&t, &alice, Claimed(40));
        l.settle_withdraw(&p, true);
        assert_eq!(l.balance_of(&t, &alice), 60);
        let s = l.solvency(&t).unwrap();
        assert_eq!((s.owed.0, s.shadow_held.0), (60, 60));
    }

    #[test]
    fn failed_withdraw_recredits() {
        let (mut l, t) = setup();
        let alice = acct("alice.near");
        l.credit(&t, &alice, Delivered::for_tests(100));
        let p = l.debit_for_withdraw(&t, &alice, Claimed(40));
        l.settle_withdraw(&p, false);
        assert_eq!(l.balance_of(&t, &alice), 100);
        assert_eq!(l.solvency(&t).unwrap().shadow_held.0, 100);
    }

    #[test]
    #[should_panic(expected = "insufficient balance")]
    fn cannot_withdraw_more_than_balance() {
        let (mut l, t) = setup();
        l.credit(&t, &acct("alice.near"), Delivered::for_tests(10));
        l.debit_for_withdraw(&t, &acct("alice.near"), Claimed(11));
    }

    /// NEAR Intents pattern: huge refund requests are capped, and finalizing
    /// never panics, so the resolve callback can't fail.
    #[test]
    fn followup_refund_is_capped() {
        let (mut l, t) = setup();
        let rich = acct("rich.near");
        l.credit(&t, &rich, Delivered::for_tests(10u128.pow(30))); // big existing balance
        let pending = l.credit_with_followup(&t, &rich, Delivered::for_tests(1));
        let refund = l.finalize_followup(&pending, 10u128.pow(35));
        assert_eq!(refund, 1, "refund must not exceed the deposit");
        let s = l.solvency(&t).unwrap();
        assert!(s.owed.0 <= s.shadow_held.0);
    }

    #[test]
    fn reconcile_detects_silent_outflow() {
        let (mut l, t) = setup();
        l.credit(&t, &acct("alice.near"), Delivered::for_tests(100));
        let ticket = l.begin_reconcile(&t);
        // The token contract really holds only 90: 10 left without the ledger knowing.
        assert_eq!(l.finish_reconcile(&ticket, 90), ReconcileOutcome::Breach { shortfall: 10 });
        assert!(matches!(l.solvency(&t).unwrap().status, BookStatus::Breach { .. }));
    }

    #[test]
    #[should_panic(expected = "withdrawals paused")]
    fn breach_blocks_withdrawals() {
        let (mut l, t) = setup();
        let alice = acct("alice.near");
        l.credit(&t, &alice, Delivered::for_tests(100));
        let ticket = l.begin_reconcile(&t);
        l.finish_reconcile(&ticket, 50);
        l.debit_for_withdraw(&t, &alice, Claimed(1));
    }

    #[test]
    fn healthy_reconcile_clears_breach() {
        let (mut l, t) = setup();
        l.credit(&t, &acct("alice.near"), Delivered::for_tests(100));
        let tk = l.begin_reconcile(&t);
        l.finish_reconcile(&tk, 50);
        let tk = l.begin_reconcile(&t);
        assert_eq!(l.finish_reconcile(&tk, 100), ReconcileOutcome::Healthy); // topped up
        assert_eq!(l.solvency(&t).unwrap().status, BookStatus::Healthy);
    }

    /// Model of async timing: token-side balance changes and ledger updates
    /// happen at different times, in the order the library enforces.
    /// Checks that reconcile never raises a false alarm, over many random runs.
    #[test]
    fn reconcile_has_no_false_alarms_under_async_timing() {
        #[derive(Clone)]
        enum Ev {
            // deposit: token balance rises first, ledger credits later
            DepositArrives(u128),
            LedgerCredit(u128),
            // withdraw: ledger debits first (done at schedule time), token later
            TokenSends(PendingOut, bool),
            ReturnArrives(PendingOut),
            LedgerSettle(PendingOut, bool),
            // reconcile: read real balance, then finish
            ReadReal(ReconcileTicket),
            Finish(ReconcileTicket, u128),
        }
        let mut seed: u64 = 0x5eed_1234;
        let mut rnd = move |n: u64| {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            (seed >> 33) % n
        };

        for run in 0..200u32 {
            let (_, t) = setup();
            // testing_env! keeps storage between runs, so each run needs its own prefix.
            let mut l = Ledger::new(format!("sim{run}").as_bytes());
            l.register_token(t.clone());
            let alice = acct("alice.near");
            let mut real: u128 = 0;
            let mut queue: Vec<(u64, Ev)> = Vec::new();
            for now in 0..300u64 {
                // schedule new actions
                match rnd(4) {
                    0 => {
                        let x = 1 + rnd(1000) as u128;
                        queue.push((now + rnd(3), Ev::DepositArrives(x)));
                    }
                    1 => {
                        let bal = l.balance_of(&t, &alice);
                        if bal > 0 {
                            let x = 1 + (rnd(bal as u64) as u128);
                            let p = l.debit_for_withdraw(&t, &alice, Claimed(x));
                            queue.push((now + 1 + rnd(3), Ev::TokenSends(p, rnd(5) != 0)));
                        }
                    }
                    2 => {
                        let tk = l.begin_reconcile(&t);
                        queue.push((now + 1 + rnd(3), Ev::ReadReal(tk)));
                    }
                    _ => {}
                }
                // process due events (any order among same-time events)
                let mut due: Vec<Ev> = Vec::new();
                queue.retain(|(at, ev)| if *at <= now { due.push(ev.clone()); false } else { true });
                for ev in due {
                    match ev {
                        Ev::DepositArrives(x) => {
                            real += x;
                            queue.push((now + 1 + rnd(3), Ev::LedgerCredit(x)));
                        }
                        Ev::LedgerCredit(x) => l.credit(&t, &alice, Delivered::for_tests(x)),
                        Ev::TokenSends(p, ok) => {
                            if ok {
                                real -= p.amount.0;
                                queue.push((now + 1 + rnd(3), Ev::LedgerSettle(p, true)));
                            } else {
                                // tokens leave, transfer fails, tokens come back, then settle
                                real -= p.amount.0;
                                queue.push((now + 1 + rnd(2), Ev::ReturnArrives(p)));
                            }
                        }
                        Ev::ReturnArrives(p) => {
                            real += p.amount.0;
                            queue.push((now + 1 + rnd(2), Ev::LedgerSettle(p, false)));
                        }
                        Ev::LedgerSettle(p, ok) => l.settle_withdraw(&p, ok),
                        Ev::ReadReal(tk) => queue.push((now + 1 + rnd(3), Ev::Finish(tk, real))),
                        Ev::Finish(tk, seen) => {
                            assert_eq!(l.finish_reconcile(&tk, seen), ReconcileOutcome::Healthy, "false alarm");
                        }
                    }
                }
            }
        }
    }
}
