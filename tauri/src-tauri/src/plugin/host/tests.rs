use super::http::perform_http_async;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::test]
async fn test_perform_http_async_get() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = [0u8; 1024];
        let _n = socket.read(&mut buf).await.unwrap();
        let response = b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nhello";
        socket.write_all(response).await.unwrap();
    });

    let url = format!("http://{}", addr);
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let result = perform_http_async(&client, "GET", &url, "").await.unwrap();
    assert_eq!(result.status, 200);
    assert_eq!(result.body, "hello");
}

#[tokio::test]
async fn test_perform_http_async_post_json() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0u8; 2048];
        let _n = socket.read(&mut buf).await.unwrap();
        let response = b"HTTP/1.1 201 Created\r\nContent-Length: 2\r\n\r\nOK";
        socket.write_all(response).await.unwrap();
    });

    let url = format!("http://{}", addr);
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let result = perform_http_async(&client, "POST", &url, r#"{"name":"Alice"}"#)
        .await
        .unwrap();
    assert_eq!(result.status, 201);
    assert_eq!(result.body, "OK");
}
