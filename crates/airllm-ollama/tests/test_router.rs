use airllm_ollama::{Complexity, ModelRouter};

#[test]
fn test_classify_low() {
    let router = ModelRouter::new();
    assert_eq!(router.classify("rename the variable"), Complexity::Low);
    assert_eq!(router.classify("format this code"), Complexity::Low);
    assert_eq!(router.classify("complete the snippet"), Complexity::Low);
    assert_eq!(router.classify("lint the project"), Complexity::Low);
}

#[test]
fn test_classify_medium() {
    let router = ModelRouter::new();
    assert_eq!(
        router.classify("implement a login handler"),
        Complexity::Medium
    );
    assert_eq!(
        router.classify("create a new test file"),
        Complexity::Medium
    );
    assert_eq!(router.classify("fix the bug in auth"), Complexity::Medium);
    assert_eq!(
        router.classify("review this pull request"),
        Complexity::Medium
    );
}

#[test]
fn test_classify_high() {
    let router = ModelRouter::new();
    assert_eq!(
        router.classify("architect the new microservices system"),
        Complexity::High
    );
    assert_eq!(
        router.classify("refactor the entire module"),
        Complexity::High
    );
    assert_eq!(
        router.classify("design the database schema"),
        Complexity::High
    );
    assert_eq!(
        router.classify("debug this complex concurrency issue"),
        Complexity::High
    );
}

#[test]
fn test_classify_cloud() {
    let router = ModelRouter::new();
    assert_eq!(
        router.classify("orchestrate the deployment pipeline"),
        Complexity::Cloud
    );
    assert_eq!(
        router.classify("plan the sprint roadmap"),
        Complexity::Cloud
    );
    assert_eq!(
        router.classify("strategy for migration"),
        Complexity::Cloud
    );
}

#[test]
fn test_classify_default_medium() {
    let router = ModelRouter::new();
    // No matching keywords → default to Medium
    assert_eq!(router.classify("hello world"), Complexity::Medium);
    assert_eq!(router.classify("do something"), Complexity::Medium);
}

#[test]
fn test_select_model() {
    let router = ModelRouter::new();
    assert_eq!(router.select_model(&Complexity::Low), "qwen3.5:4b");
    assert_eq!(router.select_model(&Complexity::Medium), "qwen3.6:27b");
    assert_eq!(
        router.select_model(&Complexity::High),
        "qwen3-coder-next:q8_0"
    );
    assert_eq!(
        router.select_model(&Complexity::Cloud),
        "qwen3.5:397b-cloud"
    );
}

#[test]
fn test_route_convenience() {
    let router = ModelRouter::new();
    assert_eq!(router.route("rename variable"), "qwen3.5:4b");
    assert_eq!(router.route("implement function"), "qwen3.6:27b");
    assert_eq!(
        router.route("architect the system"),
        "qwen3-coder-next:q8_0"
    );
    assert_eq!(
        router.route("plan the strategy"),
        "qwen3.5:397b-cloud"
    );
}

#[test]
fn test_classify_case_insensitive() {
    let router = ModelRouter::new();
    assert_eq!(router.classify("RENAME the file"), Complexity::Low);
    assert_eq!(
        router.classify("IMPLEMENT the endpoint"),
        Complexity::Medium
    );
    assert_eq!(
        router.classify("ARCHITECT the solution"),
        Complexity::High
    );
}

#[test]
fn test_default_trait() {
    let router = ModelRouter::default();
    assert_eq!(router.route("format code"), "qwen3.5:4b");
}