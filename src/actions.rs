use bitcoin::Amount;

use crate::{
    message::PayjoinProposal,
    wallet::{PaymentObligationData, PaymentObligationId, WalletHandle, WalletHandleMut, WalletId},
    Simulation, TimeStep,
};

/// An Action a wallet can perform
pub(crate) enum Action {
    UnilateralSpend(PaymentObligationId),
    InitiatePayjoin(PaymentObligationId),
    ParticipateInPayjoin(PayjoinProposal),
    Wait,
}

/// Hypothetical outcomes of an action
pub(crate) enum Event {
    PaymentObligationHandled(PaymentObligationId),
    PaymentObligationMissed(PaymentObligationId),
    PayjoinProposalHandled(PayjoinProposal),
    PayjoinOppurtunityMissed(PayjoinProposal),
    PayjoinProposalCreated(PayjoinProposal),
    PayjoinProposalMissed,
}

pub(crate) trait EventCost {
    fn cost(&self, event: &Event) -> i32;
}

struct PaymentObligationMissedCost {
    payment_missed_lambda: f64,
}

struct EventEpisode<S: EventCost>(Vec<S>);

impl<S: EventCost> EventCost for EventEpisode<S> {
    fn cost(&self, event: &Event) -> i32 {
        self.0.iter().map(|e| e.cost(event)).sum()
    }
}

// TODO: implement EventCost for each event, each trait impl should define its own lambda weights

// Each strategy will prioritize some specific actions over other to minimize its wallet cost function
// E.g the unilateral spender associates high cost with batched transaction perhaps bc they dont like interactivity and don't care much for privacy
// They want to ensure they never miss a deadline. In that case the weights behind their deadline misses are high and batched payments will be low. i.e high payment anxiety
// TODO: should strategies do more than one thing per timestep?

// Cost function should evalute over unhandled payment obligations and payjoin / cospend oppurtunities. i.e Given all the payment obligations

/// State of the wallet that can be used to potential enumerate actions
#[derive(Debug, Default)]
pub(crate) struct WalletView {
    payment_obligations: Vec<PaymentObligationData>,
    current_timestep: TimeStep,
    // TODO: utxos, feerate, cospend oppurtunities, etc.
}

impl WalletView {
    pub(crate) fn new(
        payment_obligations: Vec<PaymentObligationData>,
        current_timestep: TimeStep,
    ) -> Self {
        Self {
            payment_obligations,
            current_timestep,
        }
    }
}

// TODO: I dont think this needs an abstract type. This can just be a function?
trait SimulateOneStep<S: EventCost> {
    fn simulate_one_step(&self, action: &Action, wallet_view: &WalletView) -> EventEpisode<S>;
}

/// Model payment obligation deadline anxiety as a cubic function of the time left.
/// The goal is to make the wallets more anxious as the deadline approaches and expires.
fn deadline_anxiety(deadline: i32, current_time: i32) -> f64 {
    // delta^3 / 50
    let time_left = deadline - current_time;
    (time_left.pow(3) as f64) / 50.0
}

/// Strategies will pick one action to minimize their cost
pub(crate) trait Strategy {
    fn enumerate_candidate_actions(&self, state: &WalletView) -> impl Iterator<Item = Action>;
    fn do_something(&self, state: &WalletHandleMut) -> Action;
    fn score_action(&self, action: &Action, wallet_handle: &WalletHandleMut) -> ActionScore;
}

pub(crate) struct UnilateralSpender {
    pub(crate) payment_obligation_utility_factor: f64,
}

#[derive(Debug, PartialEq, PartialOrd)]
pub(crate) struct ActionScore(f64);

impl Eq for ActionScore {}

impl Ord for ActionScore {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        assert!(!self.0.is_nan() && !other.0.is_nan());
        self.0.partial_cmp(&other.0).expect("Checked for NaNs")
    }
}

impl Strategy for UnilateralSpender {
    /// The decision space of the unilateral spender is the set of all payment obligations and payjoin proposals
    fn enumerate_candidate_actions(&self, state: &WalletView) -> impl Iterator<Item = Action> {
        if state.payment_obligations.is_empty() {
            return vec![Action::Wait].into_iter();
        }
        let mut actions = vec![];
        for po in state.payment_obligations.iter() {
            // For every payment obligation, we can spend it unilaterally
            actions.push(Action::UnilateralSpend(po.id));
        }

        actions.into_iter()
    }

    fn do_something(&self, state: &WalletHandleMut) -> Action {
        let wallet_view = state.wallet_view();
        if wallet_view.payment_obligations.is_empty() {
            return Action::Wait;
        }
        // Unilateral spender ignores any payjoin or cospend oppurtunities
        self.enumerate_candidate_actions(&wallet_view)
            .min_by_key(|action| self.score_action(action, state))
            .unwrap_or(Action::Wait)
    }

    fn score_action(&self, action: &Action, wallet_handle: &WalletHandleMut) -> ActionScore {
        let old_info = wallet_handle.info().clone();
        let old_balance = wallet_handle.handle().effective_balance();

        // Deep clone the simulation
        let mut sim = wallet_handle.sim.clone();
        let mut predicated_wallet_handle = wallet_handle.data().id.with_mut(&mut sim);
        // Simulate the action
        predicated_wallet_handle.do_action(action);
        let new_info = wallet_handle.info().clone();
        let new_balance = wallet_handle.handle().effective_balance();

        // Compute the difference in the handled payment obligations
        let handled_payment_obligations_diff = old_info
            .handled_payment_obligations
            .difference(new_info.handled_payment_obligations.clone());

        let amount_handled = handled_payment_obligations_diff
            .iter()
            .map(|po| po.with(&sim).data().amount)
            .sum::<Amount>();

        // Intuitively the utlity gained from this action should dominate the balance lost
        // TODO; utility should be higher if the payment obligation is due soon
        let diff = old_balance - new_balance;
        // Action score is a satoshi amount but represents as a f64 for convenience. We should round and cast later.
        ActionScore(
            diff.to_float_in(bitcoin::Denomination::Satoshi)
                - (self.payment_obligation_utility_factor
                    * amount_handled.to_float_in(bitcoin::Denomination::Satoshi)),
        )
    }
}
