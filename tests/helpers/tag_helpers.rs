#![allow(dead_code)]
use oxidesk::models::Tag;
use oxidesk::database::Database;

/// Create a test tag
pub async fn create_test_tag(db: &Database, name: &str, description: Option<String>, color: Option<String>) -> Tag {
    let tag = Tag::new(name.to_string(), description, color);
    db.create_tag(&tag).await.expect("Failed to create test tag");
    tag
}

/// Create multiple test tags
pub async fn create_test_tags(db: &Database, tags: Vec<(&str, Option<&str>, Option<&str>)>) -> Vec<Tag> {
    let mut created_tags = Vec::new();
    for (name, desc, color) in tags {
        let description = desc.map(|s| s.to_string());
        let color_value = color.map(|s| s.to_string());
        let tag = create_test_tag(db, name, description, color_value).await;
        created_tags.push(tag);
    }
    created_tags
}
