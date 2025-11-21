use crate::{
    message::{MessageId, PayjoinProposal},
    wallet::{PaymentObligationData, PaymentObligationId},
    Simulation,
};

/// An Action a wallet can perform
pub(crate) enum Action {
    UnilateralSpend(PaymentObligationId),
    InitiatePayjoin(PaymentObligationId),
    ParticipateInPayjoin((MessageId, PayjoinProposal)),
    DoNothing,
}

// Each strategy will prioritize some specific actions over other to minimize its wallet cost function
// E.g the unilateral spender associates high cost with batched transaction perhaps bc they dont like interactivity and don't care much for privacy
// They want to ensure they never miss a deadline. In that case the weights behind their deadline misses are high and batched payments will be low. i.e high payment anxiety
// TODO: should strategies do more than one thing per timestep?

// Cost function should evalute over unhandled payment obligations and payjoin / cospend oppurtunities. i.e Given all the payment obligations

#[derive(Debug, Default)]
pub(crate) struct WalletPotentialActions {
    pub(crate) payment_obligations: Vec<PaymentObligationData>,
    pub(crate) payjoin_opportunities: Vec<PayjoinProposal>,
}

impl WalletPotentialActions {
    pub(crate) fn new(
        payment_obligations: Vec<PaymentObligationData>,
        payjoin_opportunities: Vec<PayjoinProposal>,
    ) -> Self {
        Self {
            payment_obligations,
            payjoin_opportunities,
        }
    }
}

/// Model payment obligation deadline anxiety as a cubic function of the time left.
/// The goal is to make the wallets more anxious as the deadline approaches and expires.
fn deadline_anxiety(deadline: i32, current_time: i32) -> f64 {
    // delta^3 / 50
    let time_left = deadline - current_time;
    (time_left.pow(3) as f64) / 50.0
}

pub(crate) trait Strategy {
    fn do_something(&self, state: &WalletPotentialActions, sim: &Simulation) -> Action;

    fn cost(&self, action: &Action, current_time: i32, sim: &Simulation) -> i32;
}

pub(crate) struct UnilateralSpender;

impl Strategy for UnilateralSpender {
    fn do_something(&self, state: &WalletPotentialActions, sim: &Simulation) -> Action {
        if state.payment_obligations.is_empty() && state.payjoin_opportunities.is_empty() {
            return Action::DoNothing;
        }
        let current_time = sim.current_timestep.0 as i32;
        // Unilateral spender ignores any payjoin or cospend oppurtunities
        let action = state
            .payment_obligations
            .iter()
            .map(|po| Action::UnilateralSpend(po.id))
            .min_by_key(|action| self.cost(action, current_time, sim));

        action.unwrap_or(Action::DoNothing)
    }

    // TODO: this should be parameterized on the state diff not the action
    fn cost(&self, action: &Action, current_time: i32, sim: &Simulation) -> i32 {
        match action {
            Action::DoNothing => i32::MIN, // Should always try do something for this strategy. Other may want to wait and do nothing for batching oppurtunities
            Action::UnilateralSpend(po) => {
                // Cost function terms should be penatly for missing payments
                let deadline = po.with(sim).data().clone().deadline;
                let payment_anxiety = deadline_anxiety(deadline.0 as i32, current_time);

                // TODO: remove magic weights. All weights should be parametrized or trait associated types
                // TODO: model a term for interactions. interaction should be interms of our deadline anxiety
                // TODO: model term for fee rate
                let score = payment_anxiety * 5.0;

                score.round() as i32
            }
            _ => unimplemented!(),
        }
    }
}

pub(crate) enum Strategies {
    UnilateralSpender(UnilateralSpender),
}
