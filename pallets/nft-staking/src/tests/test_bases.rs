use super::*;

#[fixture]
pub fn ext() -> TestExternalities {
	new_test_ext()
}

#[template]
#[rstest]
#[case::dpos(PermissionType::DPoS)]
pub fn dpos(mut ext: TestExternalities, #[case] permission: PermissionType) {}

#[template]
#[rstest]
#[case::pos(PermissionType::PoS)]
pub fn pos(mut ext: TestExternalities, #[case] permission: PermissionType) {}

#[template]
#[rstest]
#[case::dpos(PermissionType::DPoS)]
#[case::pos(PermissionType::PoS)]
pub fn permission_cases(mut ext: TestExternalities, #[case] permission: PermissionType) {}

#[template]
#[rstest]
#[case::no_force(false)]
#[case::force(true)]
pub fn force_cases(mut ext: TestExternalities, #[case] force: bool) {}

#[template]
#[rstest]
#[case::dpos_no_force(PermissionType::DPoS, false)]
#[case::dpos_force(PermissionType::DPoS, true)]
#[case::pos_no_force(PermissionType::PoS, false)]
#[case::pos_force(PermissionType::PoS, true)]
pub fn permission_x_force_cases(
	mut ext: TestExternalities,
	#[case] permission: PermissionType,
	#[case] force: bool,
) {
}
