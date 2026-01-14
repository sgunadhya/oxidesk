mod helpers;

use helpers::*;
use oxidesk::{
    models::{Contact, ContactChannel, User, UserType},
    services::validate_and_normalize_email,
};

#[tokio::test]
async fn test_contact_creation_success() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create a contact
    let email = validate_and_normalize_email("contact@example.com").unwrap();
    let user = User::new(email.clone(), UserType::Contact);
    let contact = Contact::new(user.id.clone(), Some("John Doe".to_string()));

    db.create_user(&user).await.unwrap();
    db.create_contact(&contact).await.unwrap();

    // Verify contact was created
    let retrieved = db
        .get_user_by_email_and_type(&email, &UserType::Contact)
        .await
        .unwrap();
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().email, email);

    // Verify contact details
    let contact_details = db.get_contact_by_user_id(&user.id).await.unwrap();
    assert!(contact_details.is_some());
    let contact_details = contact_details.unwrap();
    assert_eq!(contact_details.first_name, Some("John Doe".to_string()));

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_duplicate_contact_email_rejection() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create first contact
    let email = validate_and_normalize_email("duplicate@example.com").unwrap();
    let user1 = User::new(email.clone(), UserType::Contact);
    let contact1 = Contact::new(user1.id.clone(), Some("Contact 1".to_string()));

    db.create_user(&user1).await.unwrap();
    db.create_contact(&contact1).await.unwrap();

    // Try to create another contact with same email (should fail)
    let user2 = User::new(email.clone(), UserType::Contact);
    let result = db.create_user(&user2).await;

    // Should fail due to per-type email uniqueness constraint
    assert!(result.is_err());

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_contact_email_can_duplicate_agent_email() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create an agent with an email
    let email = validate_and_normalize_email("shared@example.com").unwrap();
    let agent_user = User::new(email.clone(), UserType::Agent);
    db.create_user(&agent_user).await.unwrap();

    // Now create a contact with the same email (should succeed due to per-type uniqueness)
    let contact_user = User::new(email.clone(), UserType::Contact);
    let contact = Contact::new(contact_user.id.clone(), Some("Shared Email".to_string()));

    let result = db.create_user(&contact_user).await;
    assert!(
        result.is_ok(),
        "Should allow same email for different user types"
    );

    db.create_contact(&contact).await.unwrap();

    // Verify both exist
    let agent = db
        .get_user_by_email_and_type(&email, &UserType::Agent)
        .await
        .unwrap();
    let contact_retrieved = db
        .get_user_by_email_and_type(&email, &UserType::Contact)
        .await
        .unwrap();

    assert!(agent.is_some());
    assert!(contact_retrieved.is_some());

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_contact_channel_linking() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create a contact
    let email = validate_and_normalize_email("channel@example.com").unwrap();
    let user = User::new(email.clone(), UserType::Contact);
    let contact = Contact::new(user.id.clone(), Some("Channel Test".to_string()));

    db.create_user(&user).await.unwrap();
    db.create_contact(&contact).await.unwrap();

    // Create a contact channel
    let inbox_id = "inbox-123".to_string();
    let channel_email = "channel@inbox.example.com".to_string();
    let channel = ContactChannel::new(contact.id.clone(), inbox_id.clone(), channel_email.clone());

    db.create_contact_channel(&channel).await.unwrap();

    // Verify channel was created
    let channels = db.get_contact_channels(&contact.id).await.unwrap();
    assert_eq!(channels.len(), 1);
    assert_eq!(channels[0].inbox_id, inbox_id);
    assert_eq!(channels[0].email, channel_email);

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_get_contact_by_id() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create a contact
    let email = validate_and_normalize_email("get@example.com").unwrap();
    let user = User::new(email.clone(), UserType::Contact);
    let contact = Contact::new(user.id.clone(), Some("Get Contact".to_string()));

    db.create_user(&user).await.unwrap();
    db.create_contact(&contact).await.unwrap();

    // Retrieve contact by user ID
    let retrieved_user = db.get_user_by_id(&user.id).await.unwrap();
    assert!(retrieved_user.is_some());

    let retrieved_user = retrieved_user.unwrap();
    assert_eq!(retrieved_user.id, user.id);
    assert_eq!(retrieved_user.email, email);
    assert!(matches!(retrieved_user.user_type, UserType::Contact));

    // Retrieve contact details
    let retrieved_contact = db.get_contact_by_user_id(&user.id).await.unwrap();
    assert!(retrieved_contact.is_some());

    let retrieved_contact = retrieved_contact.unwrap();
    assert_eq!(
        retrieved_contact.first_name,
        Some("Get Contact".to_string())
    );

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_contact_without_name() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create a contact without a name
    let email = validate_and_normalize_email("noname@example.com").unwrap();
    let user = User::new(email.clone(), UserType::Contact);
    let contact = Contact::new(user.id.clone(), None);

    db.create_user(&user).await.unwrap();
    db.create_contact(&contact).await.unwrap();

    // Verify contact was created
    let retrieved = db.get_contact_by_user_id(&user.id).await.unwrap();
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().first_name, None);

    teardown_test_db(test_db).await;
}
