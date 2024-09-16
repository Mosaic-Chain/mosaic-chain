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
