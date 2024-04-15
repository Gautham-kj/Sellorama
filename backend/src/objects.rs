use anyhow::Error;
use aws_config::BehaviorVersion;
use aws_sdk_s3::{
    config::{Credentials, Region, SharedCredentialsProvider},
    primitives::ByteStream,
    Client,
};

pub struct S3Credentials {
    access_key: String,
    secret_access_key: String,
    session_token: Option<String>,
    expires_after: Option<std::time::SystemTime>,
    endpoint_url: String,
}

impl S3Credentials {
    pub fn new(
        access_key: String,
        secret_access_key: String,
        session_token: Option<String>,
        expires_after: Option<std::time::SystemTime>,
        endpoint_url: String,
    ) -> Self {
        return S3Credentials {
            access_key: access_key,
            secret_access_key: secret_access_key,
            session_token: session_token,
            expires_after: expires_after,
            endpoint_url: endpoint_url,
        };
    }
}

pub async fn get_s3_client(region: String, credentials: S3Credentials) -> Result<Client, Error> {
    // let provider_name = credentials.provider_name.clone().as_str();
    let s3_credentials = Credentials::new(
        credentials.access_key,
        credentials.secret_access_key,
        credentials.session_token,
        credentials.expires_after,
        "Sellorama_s3_client",
    );

    let credentials_provider = SharedCredentialsProvider::new(s3_credentials);

    let mut s3_config = aws_config::load_defaults(BehaviorVersion::latest()).await;
    s3_config = s3_config
        .into_builder()
        .credentials_provider(credentials_provider)
        .region(Region::new(region))
        .endpoint_url(credentials.endpoint_url)
        .build();

    let client = Client::new(&s3_config);
    Ok(client)
}

pub async fn get_presigned_url(
    client: &Client,
    bucket: &str,
    object: &str,
    expires_in: u64,
) -> Result<String, Box<dyn std::error::Error>> {
    let expires_in = std::time::Duration::from_secs(expires_in);
    let presigned_request = client
        .get_object()
        .bucket(bucket)
        .key(object)
        .presigned(aws_sdk_s3::presigning::PresigningConfig::expires_in(
            expires_in,
        )?)
        .await?;

    Ok(presigned_request.uri().to_owned())
}

pub async fn put_object(
    client: &Client,
    bucket: &str,
    object: String,
    data: ByteStream,
) -> Result<(), Box<dyn std::error::Error>> {
    let _response = client
        .put_object()
        .bucket(bucket)
        .key(object.as_str())
        .body(data)
        .send()
        .await?;

    Ok(())
}
