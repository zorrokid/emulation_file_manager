use axum::{
    Router,
    body::Body,
    http::{
        HeaderMap, HeaderValue, StatusCode,
        header::{CONTENT_DISPOSITION, CONTENT_TYPE},
    },
    routing::get,
};
use bytes::Bytes;
use core_types::events::HttpDownloadEvent;
use flume::{Receiver, Sender};
use futures::StreamExt;
use http_downloader::{DownloadError, download_file};
use tempfile::{TempDir, tempdir};
use tokio::{net::TcpListener, task::JoinHandle};
use tokio::time::Duration;

struct TestContext {
    base_url: String,
    server: JoinHandle<()>,
    output_dir: TempDir,
    progress_tx: Sender<HttpDownloadEvent>,
    progress_rx: Receiver<HttpDownloadEvent>,
    cancel_tx: Sender<()>,
    cancel_rx: Receiver<()>,
}

impl TestContext {
    fn abort_server(&self) {
        self.server.abort();
    }
}

async fn start_test_server(app: Router) -> (String, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("test server should bind to an ephemeral localhost port");
    let addr = listener
        .local_addr()
        .expect("test server should expose its bound address");

    let server = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("test server should run until the test ends");
    });

    (format!("http://{addr}"), server)
}

fn create_output_dir() -> TempDir {
    tempdir().expect("test output directory should be created")
}

fn create_progress_channel() -> (Sender<HttpDownloadEvent>, Receiver<HttpDownloadEvent>) {
    flume::unbounded()
}

fn create_cancel_channel() -> (Sender<()>, Receiver<()>) {
    flume::unbounded()
}

fn drain_events(progress_rx: &Receiver<HttpDownloadEvent>) -> Vec<HttpDownloadEvent> {
    progress_rx.try_iter().collect()
}

fn build_fixture_app(stream_chunk_delay: Duration) -> Router {
    Router::new()
        .route("/header-file", get(header_file_handler))
        .route("/url-fallback.bin", get(url_fallback_handler))
        .route("/invalid-header", get(invalid_header_handler))
        .route("/http-error", get(http_error_handler))
        .route(
            "/stream.bin",
            get(move || async move { streamed_download_handler(stream_chunk_delay).await }),
        )
}

async fn header_file_handler() -> (HeaderMap, Body) {
    (
        attachment_headers("fixture-download.zip"),
        Body::from("fixture body"),
    )
}

async fn url_fallback_handler() -> (HeaderMap, Body) {
    (binary_headers(), Body::from("fallback body"))
}

async fn invalid_header_handler() -> (HeaderMap, Body) {
    (
        invalid_content_disposition_headers(),
        Body::from("invalid header body"),
    )
}

async fn http_error_handler() -> (StatusCode, HeaderMap, Body) {
    (
        StatusCode::NOT_FOUND,
        binary_headers(),
        Body::from("missing fixture"),
    )
}

async fn streamed_download_handler(stream_chunk_delay: Duration) -> (HeaderMap, Body) {
    let chunks = vec![
        Ok::<Bytes, std::io::Error>(Bytes::from_static(b"stream-1")),
        Ok(Bytes::from_static(b"stream-2")),
        Ok(Bytes::from_static(b"stream-3")),
        Ok(Bytes::from_static(b"stream-4")),
    ];

    let stream = futures::stream::iter(chunks).then(move |chunk| {
        let delay = stream_chunk_delay;
        async move {
            tokio::time::sleep(delay).await;
            chunk
        }
    });

    (
        attachment_headers("streamed-download.bin"),
        Body::from_stream(stream),
    )
}

fn attachment_headers(file_name: &str) -> HeaderMap {
    let mut headers = binary_headers();
    headers.insert(
        CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!("attachment; filename=\"{file_name}\""))
            .expect("fixture Content-Disposition header should be valid"),
    );
    headers
}

fn invalid_content_disposition_headers() -> HeaderMap {
    let mut headers = binary_headers();
    headers.insert(
        CONTENT_DISPOSITION,
        HeaderValue::from_static("attachment; filename*=UTF-8''%FF"),
    );
    headers
}

fn binary_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/octet-stream"),
    );
    headers
}

async fn setup_download_test(stream_chunk_delay: Duration) -> TestContext {
    let fixture_app = build_fixture_app(stream_chunk_delay);
    let (base_url, server) = start_test_server(fixture_app).await;
    let output_dir = create_output_dir();
    let (progress_tx, progress_rx) = create_progress_channel();
    let (cancel_tx, cancel_rx) = create_cancel_channel();

    TestContext {
        base_url,
        server,
        output_dir,
        progress_tx,
        progress_rx,
        cancel_tx,
        cancel_rx,
    }
}

#[tokio::test]
async fn test_download_file_successful_download_sends_started_progress_completed() {
    let test_context = setup_download_test(Duration::from_millis(5)).await;

    let result = download_file(
        &format!("{}/header-file", test_context.base_url),
        test_context.output_dir.path(),
        &test_context.progress_tx,
        &test_context.cancel_rx,
    )
    .await;

    test_context.abort_server();

    let result = match result {
        Ok(result) => result,
        Err(error) => panic!("download should succeed: {error}"),
    };
    assert_eq!(
        result.file_path.file_name().and_then(|name| name.to_str()),
        Some("fixture-download.zip")
    );

    let file_contents = tokio::fs::read_to_string(&result.file_path)
        .await
        .expect("downloaded fixture file should be readable");
    assert_eq!(file_contents, "fixture body");

    let events = drain_events(&test_context.progress_rx);
    let started_index = events
        .iter()
        .position(|event| matches!(event, HttpDownloadEvent::Started { .. }))
        .expect("Started event should be emitted");
    let progress_index = events
        .iter()
        .position(|event| matches!(event, HttpDownloadEvent::Progress { .. }))
        .expect("Progress event should be emitted");
    let completed_index = events
        .iter()
        .position(|event| matches!(event, HttpDownloadEvent::Completed { .. }))
        .expect("Completed event should be emitted");

    assert!(started_index < progress_index);
    assert!(progress_index < completed_index);

    let completed_path = events.iter().find_map(|event| match event {
        HttpDownloadEvent::Completed { file_path } => Some(file_path.clone()),
        _ => None,
    });
    assert_eq!(completed_path, Some(result.file_path));
}

#[tokio::test]
async fn test_download_file_missing_content_disposition_falls_back_to_url_filename() {
    let test_context = setup_download_test(Duration::from_millis(5)).await;

    let result = download_file(
        &format!("{}/url-fallback.bin", test_context.base_url),
        test_context.output_dir.path(),
        &test_context.progress_tx,
        &test_context.cancel_rx,
    )
    .await;

    test_context.abort_server();

    let result = match result {
        Ok(result) => result,
        Err(error) => panic!("download should succeed: {error}"),
    };

    assert_eq!(
        result.file_path.file_name().and_then(|name| name.to_str()),
        Some("url-fallback.bin")
    );

    let file_contents = tokio::fs::read_to_string(&result.file_path)
        .await
        .expect("downloaded fallback file should be readable");
    assert_eq!(file_contents, "fallback body");
}

#[tokio::test]
async fn test_download_file_invalid_content_disposition_returns_filename_error() {
    let test_context = setup_download_test(Duration::from_millis(5)).await;

    let result = download_file(
        &format!("{}/invalid-header", test_context.base_url),
        test_context.output_dir.path(),
        &test_context.progress_tx,
        &test_context.cancel_rx,
    )
    .await;

    test_context.abort_server();

    assert!(matches!(result, Err(DownloadError::FilenameError(_))));
}

#[tokio::test]
async fn test_download_file_non_success_response_returns_request_error() {
    let test_context = setup_download_test(Duration::from_millis(5)).await;

    let result = download_file(
        &format!("{}/http-error", test_context.base_url),
        test_context.output_dir.path(),
        &test_context.progress_tx,
        &test_context.cancel_rx,
    )
    .await;

    test_context.abort_server();

    assert!(matches!(result, Err(DownloadError::RequestFailed(_))));

    let mut directory_entries = tokio::fs::read_dir(test_context.output_dir.path())
        .await
        .expect("output directory should remain readable");
    assert!(
        directory_entries
            .next_entry()
            .await
            .expect("directory iteration should succeed")
            .is_none(),
        "non-success responses should not leave files behind"
    );
}

#[tokio::test]
async fn test_download_file_cancellation_removes_partial_file_and_returns_cancelled() {
    let test_context = setup_download_test(Duration::from_millis(50)).await;
    let stream_url = format!("{}/stream.bin", test_context.base_url);

    let download_future = download_file(
        &stream_url,
        test_context.output_dir.path(),
        &test_context.progress_tx,
        &test_context.cancel_rx,
    );
    let cancel_future = async {
        let mut seen_events = Vec::new();

        loop {
            let event = test_context
                .progress_rx
                .recv_async()
                .await
                .expect("download should emit progress events before cancellation");
            let should_cancel = matches!(event, HttpDownloadEvent::Progress { .. });
            seen_events.push(event);

            if should_cancel {
                test_context
                    .cancel_tx
                    .send(())
                    .expect("cancellation signal should be sent successfully");
                break;
            }
        }

        seen_events
    };

    let (result, mut observed_events) = tokio::join!(download_future, cancel_future);

    test_context.abort_server();

    assert!(matches!(result, Err(DownloadError::Cancelled)));

    observed_events.extend(drain_events(&test_context.progress_rx));
    assert!(
        !observed_events
            .iter()
            .any(|event| matches!(event, HttpDownloadEvent::Completed { .. })),
        "cancelled downloads should not emit Completed"
    );

    let mut directory_entries = tokio::fs::read_dir(test_context.output_dir.path())
        .await
        .expect("output directory should remain readable");
    assert!(
        directory_entries
            .next_entry()
            .await
            .expect("directory iteration should succeed")
            .is_none(),
        "cancelled downloads should remove partial files"
    );
}
