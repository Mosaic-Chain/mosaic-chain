use sdk::{frame_system, pallet_collective, pallet_membership, sp_runtime};

use super::{
	parameter_types, params, AccountId, BlockWeights, EitherOfDiverse, Runtime, RuntimeCall,
	RuntimeEvent, RuntimeOrigin, Weight,
};

// Shared values
parameter_types! {
	pub const MaxProposals: u32 = 10;
	pub const MaxMembers: u32 = 100;
	pub MaxProposalWeight: Weight = sp_runtime::Perbill::from_percent(50) * BlockWeights::get().max_block;
}

pub type CouncilOrigin = EitherOfDiverse<
	frame_system::EnsureRoot<AccountId>,
	pallet_collective::EnsureProportionAtLeast<AccountId, council::CollectiveInstance, 2, 3>,
>;

macro_rules! impl_collective {
	($collective:ident, $instance:ident) => {
		pub mod $collective {
			use super::*;

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
			type MaxMembers = MaxMembers;
			type WeightInfo = pallet_membership::weights::SubstrateWeight<Runtime>;
		}

		impl pallet_collective::Config<$collective::CollectiveInstance> for Runtime {
			type RuntimeOrigin = RuntimeOrigin;
			type Proposal = RuntimeCall;
			type RuntimeEvent = RuntimeEvent;

			type MotionDuration = $collective::MotionDuration;
			type MaxProposals = MaxProposals;
			type MaxMembers = MaxMembers;

			type DefaultVote = pallet_collective::MoreThanMajorityThenPrimeDefaultVote;
			type WeightInfo = pallet_collective::weights::SubstrateWeight<Runtime>;
			type SetMembersOrigin = frame_system::EnsureRoot<AccountId>;
			type MaxProposalWeight = MaxProposalWeight;
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
