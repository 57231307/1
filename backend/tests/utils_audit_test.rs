use bingxi_backend::utils::audit::*;


#[test]
fn test_security_event_display() {
    assert_eq!(SecurityEvent::ResetPassword.to_string(), "RESET_PASSWORD");
    assert_eq!(
        SecurityEvent::AuthorizationDenied.to_string(),
        "AUTHORIZATION_DENIED"
    );
    assert_eq!(SecurityEvent::UserDeleted.to_string(), "USER_DELETED");
    assert_eq!(
        SecurityEvent::TestDatabaseConnection.to_string(),
        "TEST_DATABASE_CONNECTION"
    );
}