use kanidm_client::KanidmClient;
use kanidmd_testkit::ADMIN_TEST_PASSWORD;
use reqwest::header::HeaderValue;

#[kanidmd_testkit::test]
async fn test_sync_account_lifecycle(rsclient: KanidmClient) {
    let a_res = rsclient
        .auth_simple_password("admin", ADMIN_TEST_PASSWORD)
        .await;
    assert!(a_res.is_ok());

    let a_list = rsclient.idm_sync_account_list().await.unwrap();
    assert!(a_list.is_empty());

    rsclient
        .idm_sync_account_create("ipa_sync_account", Some("Demo of a sync account"))
        .await
        .unwrap();

    let a_list = rsclient.idm_sync_account_list().await.unwrap();
    assert!(!a_list.is_empty());

    let a = rsclient
        .idm_sync_account_get("ipa_sync_account")
        .await
        .unwrap();
    assert!(a.is_some());
    println!("{:?}", a);

    // Get a token

    // List sessions?

    // Reset Sign Key
    // Get New token

    // Get sync state

    // Delete session

    // Sync state fails.

    // Delete account
}


#[kanidmd_testkit::test]
async fn test_scim_sync_get(rsclient: KanidmClient) {
    // We need to do manual reqwests here.
    let addr = rsclient.get_url();

    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {:?}", rsclient.get_token().await)).unwrap(),
    );

    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .default_headers(headers)
        .build()
        .unwrap();
    // here we test the /ui/ endpoint which should have the headers
    let response = match client.get(format!("{}/scim/v1/Sync", addr)).send().await {
        Ok(value) => value,
        Err(error) => {
            panic!("Failed to query {:?} : {:#?}", addr, error);
        }
    };
    eprintln!("response: {:#?}", response);
    // assert_eq!(response.status(), 200);

    // eprintln!(
    //     "csp headers: {:#?}",
    //     response.headers().get("content-security-policy")
    // );
    // assert_ne!(response.headers().get("content-security-policy"), None);
    // eprintln!("{}", response.text().await.unwrap());
}