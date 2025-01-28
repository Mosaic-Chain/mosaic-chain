use sdk::{frame_support, frame_system, pallet_collective, pallet_membership};

use frame_support::traits::EitherOfDiverse;

use super::{params, weights, AccountId, Runtime, RuntimeCall, RuntimeEvent, RuntimeOrigin};

// NOTE: changing this to a type which won't check if a vote happened is a risk, because
// pallet_collective::Call::propose{} with < 2 threshold will result in an immediate pallet_collective::Call::execute{}
// basically turning all of the council members into a "dictator" account.
//
// This ensures that a vote with 2/3 aye ratio is needed for a doas proposal to be accepted.
pub type CouncilOrigin = EitherOfDiverse<
	frame_system::EnsureRoot<AccountId>,
	pallet_collective::EnsureProportionAtLeast<AccountId, council::CollectiveInstance, 2, 3>,
>;

macro_rules! impl_collective {
	($collective:ident, $instance:ident) => {
		pub mod $collective {
			use super::*;

			pub use params::constant::collective::*;
			pub use params::dynamic::$collective::*;

			pub type MembershipInstance = pallet_membership::$instance;
			pub type CollectiveInstance = pallet_collective::$instance;
		}

		impl pallet_membership::Config<$collective::MembershipInstance> for Runtime {
			type RuntimeEvent = RuntimeEvent;
			type AddOrigin = CouncilOrigin;
			type RemoveOrigin = CouncilOrigin;
			type SwapOrigin = CouncilOrigin;
			type ResetOrigin = CouncilOrigin;
			type PrimeOrigin = CouncilOrigin;
			type MembershipInitialized =
				pallet_collective::Pallet<Runtime, $collective::CollectiveInstance>;
			type MembershipChanged =
				pallet_collective::Pallet<Runtime, $collective::CollectiveInstance>;
			type MaxMembers = params::constant::membership::MaxMembers;
			type WeightInfo = weights::pallet::membership::Weights<Self>;
		}

		impl pallet_collective::Config<$collective::CollectiveInstance> for Runtime {
			type RuntimeOrigin = RuntimeOrigin;
			type DisapproveOrigin = CouncilOrigin;
			type KillOrigin = CouncilOrigin;

			type Consideration = ();

			type Proposal = RuntimeCall;
			type RuntimeEvent = RuntimeEvent;

			type MotionDuration = $collective::MotionDuration;
			type MaxProposals = params::constant::collective::MaxProposals;
			type MaxMembers = params::constant::collective::MaxMembers;

			type DefaultVote = pallet_collective::MoreThanMajorityThenPrimeDefaultVote;
			type SetMembersOrigin = CouncilOrigin;
			type MaxProposalWeight = params::constant::collective::MaxProposalWeight;
			type WeightInfo = weights::pallet::collective::Weights<Self>;
		}
	};
}

impl_collective!(council, Instance1);
impl_collective!(development_collective, Instance2);
impl_collective!(financial_collective, Instance3);
impl_collective!(community_collective, Instance4);
impl_collective!(team_and_advisors_collective, Instance5);
impl_collective!(security_collective, Instance6);
impl_collective!(education_collective, Instance7);
