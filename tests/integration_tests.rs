#![allow(clippy::all)]
pub mod common;
pub mod compliance;

#[tokio::test]
async fn test_compliance_suite() {
    let ctx = common::TestContext::new().await;

    compliance::t01_auth_session::run(&ctx).await;
    compliance::t02_rbac::run(&ctx).await;
    compliance::t03_crud_schema::run(&ctx).await;
    compliance::t04_dynamic_filters::run(&ctx).await;
    compliance::t05_audit_logs::run(&ctx).await;
    compliance::t06_soft_delete::run(&ctx).await;
    compliance::t07_observability::run(&ctx).await;
    compliance::t08_rate_limit::run(&ctx).await;
    compliance::t09_status::run(&ctx).await;
    compliance::t10_role_features::run(&ctx).await;
    compliance::t11_session_invalidation::run(&ctx).await;
    compliance::t12_error_logs::run(&ctx).await;
    compliance::t13_pdf_debug::run(&ctx).await;
    compliance::t14_audit_explorer::run(&ctx).await;
    compliance::t15_bootstrap::run(&ctx).await;
    compliance::t16_dashboard::run(&ctx).await;
    compliance::t17_upload::run(&ctx).await;
    compliance::t18_crud_errors::run(&ctx).await;
}
