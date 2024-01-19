use std::fs::OpenOptions;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, bail};

use aws_config::BehaviorVersion;
use aws_sdk_s3::Client as S3Client;

use aws_sdk_s3::primitives::ByteStream;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use log::{error, info};
use tar::Archive;

use crate::index::ckv::CKVIndex;
use crate::proto::generated_proto::common::IKVStoreConfig;
use crate::utils;

pub fn load_index(config: &IKVStoreConfig) -> anyhow::Result<()> {
    let working_mount_directory = crate::utils::paths::get_working_mount_directory_fqn(config)?;
    let index_mount_directory = crate::utils::paths::get_index_mount_directory_fqn(config)?;

    // create paths if not exists
    std::fs::create_dir_all(&working_mount_directory)?;
    std::fs::create_dir_all(&index_mount_directory)?;

    // TODO: we might need to load a new base index based on age (for compaction!!)
    let mut download = false;

    if CKVIndex::index_not_present(config)? {
        info!("No base index present in mount directory.");
        CKVIndex::delete_all(config)?;
        download = true;
    } else if let Err(e) = CKVIndex::is_valid_index(config) {
        info!(
            "Base index found in inconsistent state: {}. Clearing out index.",
            e
        );
        CKVIndex::delete_all(config)?;
        download = true;
    }

    if download {
        info!("Attempting to download from S3 repository.");
        orchestrate_index_download(&working_mount_directory, &index_mount_directory, config)?;
    }

    Ok(())
}

pub fn upload_index(config: &IKVStoreConfig) -> anyhow::Result<()> {
    let working_mount_directory = crate::utils::paths::get_working_mount_directory_fqn(config)?;
    let index_mount_directory = crate::utils::paths::get_index_mount_directory_fqn(config)?;
    // create paths if not exists
    std::fs::create_dir_all(&working_mount_directory)?;
    std::fs::create_dir_all(&index_mount_directory)?;

    // check if index exists, error if not
    if let Err(e) = CKVIndex::is_valid_index(config) {
        bail!("Cannot upload bad index, error: {}", e);
    }

    // upload as base index
    orchestrate_index_upload(&working_mount_directory, &index_mount_directory, config)
}

#[tokio::main(flavor = "current_thread")]
async fn orchestrate_index_upload(
    working_mount_directory: &str,
    index_mount_directory: &str,
    config: &IKVStoreConfig,
) -> anyhow::Result<()> {
    let tarball_index_filename = format!("{}/base_index.tar.gz", working_mount_directory);

    // clear any old base index tar archives
    if Path::new(&tarball_index_filename).exists() {
        std::fs::remove_file(&tarball_index_filename)?;
    }

    pack_tarball(index_mount_directory, &tarball_index_filename)?;

    // https://docs.aws.amazon.com/AmazonS3/latest/userguide/example_s3_PutObject_section.html
    // TODO: need to handle large file uploads!!
    // https://docs.aws.amazon.com/AmazonS3/latest/userguide/example_s3_Scenario_UsingLargeFiles_section.html

    let aws_config = aws_config::defaults(BehaviorVersion::latest())
        .region("eu-north-1")
        .load()
        .await;
    let client = S3Client::new(&aws_config);

    // ikv-base-indexes-v1
    let bucket_name = config
        .stringConfigs
        .get("base_index_s3_bucket_name")
        .ok_or(anyhow!(
            "base_index_s3_bucket_name is a required gateway-specified config"
        ))?
        .to_string();

    let s3_key_prefix = config
        .stringConfigs
        .get("base_index_s3_bucket_prefix")
        .ok_or(anyhow!(
            "base_index_s3_bucket_prefix is a required gateway-specified config"
        ))?
        .to_string();

    let partition = config
        .intConfigs
        .get("partition")
        .copied()
        .ok_or(anyhow!("partition is a required user-specified config"))?;

    let epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_millis()
        .to_string();

    // key: <account_id><storename>/<epoch>/<partition>
    let base_index_s3_key = format!("{}/{}/{}", &s3_key_prefix, &epoch, partition);

    // upload!
    let sse_key_objects = utils::encryption::sse_key_and_digest(config)?;
    let body = ByteStream::from_path(Path::new(&tarball_index_filename)).await?;

    client
        .put_object()
        // https://docs.aws.amazon.com/AmazonS3/latest/userguide/ServerSideEncryptionCustomerKeys.html
        .sse_customer_algorithm(aws_sdk_s3::types::ServerSideEncryption::Aes256.as_str())
        .sse_customer_key(sse_key_objects.0)
        .sse_customer_key_md5(sse_key_objects.1)
        .bucket(bucket_name)
        .key(base_index_s3_key)
        .body(body)
        .send()
        .await?;

    // Remove tarball
    std::fs::remove_file(tarball_index_filename)?;
    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn orchestrate_index_download(
    working_mount_directory: &str,
    index_mount_directory: &str,
    config: &IKVStoreConfig,
) -> anyhow::Result<()> {
    // References:
    // https://docs.aws.amazon.com/AmazonS3/latest/userguide/example_s3_Scenario_UsingLargeFiles_section.html
    // https://docs.aws.amazon.com/AmazonS3/latest/userguide/example_s3_ListObjects_section.html

    // TODO: client initialization
    // Ref: https://docs.rs/aws-config/latest/aws_config/ecs/index.html
    // https://docs.aws.amazon.com/sdk-for-rust/latest/dg/getting-started.html#getting-started-step1

    let aws_config = aws_config::defaults(BehaviorVersion::latest())
        .region("eu-north-1")
        .load()
        .await;
    let client = S3Client::new(&aws_config);

    //let aws_config = aws_config::load_from_env().await;
    //let client = S3Client::new(&aws_config);

    let bucket_name = config
        .stringConfigs
        .get("base_index_s3_bucket_name")
        .ok_or(anyhow!(
            "base_index_s3_bucket_name is a required gateway-specified config"
        ))?
        .to_string();

    // <account-id>/<store-name>
    let s3_key_prefix = config
        .stringConfigs
        .get("base_index_s3_bucket_prefix")
        .ok_or(anyhow!(
            "base_index_s3_bucket_prefix is a required gateway-specified config"
        ))?
        .to_string();

    let partition = config
        .intConfigs
        .get("partition")
        .copied()
        .ok_or(anyhow!("partition is a required client-specified config"))?;

    // list objects based on prefix
    let mut response = client
        .list_objects_v2()
        .bucket(bucket_name.clone())
        .max_keys(3)
        .prefix(&s3_key_prefix)
        .into_paginator()
        .send();

    let mut base_index_key: Option<String> = None;
    let mut max_epoch = u128::MIN;

    while let Some(result) = response.next().await {
        for object in result?.contents() {
            // key format: <account_id>/<storename>/<epoch>/<partition>
            if let Some(key) = object.key() {
                let parts = key.split('/').collect::<Vec<&str>>();

                let key_partition = parts.get(3).ok_or(anyhow!(
                    "malformed base index key: {}, expecting partition",
                    key
                ))?;
                let key_partition: i64 = key_partition.parse::<i64>()?;

                if partition != key_partition {
                    continue;
                }

                let key_epoch = parts.get(2).ok_or(anyhow!(
                    "malformed base index key: {}, expecting epoch",
                    key
                ))?;
                let key_epoch: u128 = key_epoch.parse::<u128>()?;
                if max_epoch < key_epoch {
                    max_epoch = key_epoch;
                    base_index_key = Some(key.to_string());
                }
            }
        }
    }

    if base_index_key.is_none() {
        info!("Did not find base index prefix: {}", &s3_key_prefix);
        return Ok(());
    }

    let key = base_index_key.unwrap();
    info!("Found base index, base-index-key: {}", &key);

    // download, unpack and delete tarred file
    let tarball_index_filename = format!("{}/base_index.tar.gz", working_mount_directory);

    // clear any old base index tar archives
    if Path::new(&tarball_index_filename).exists() {
        std::fs::remove_file(&tarball_index_filename)?;
    }

    download_from_s3(&client, config, &bucket_name, &key, &tarball_index_filename).await?;
    unpack_tarball(&tarball_index_filename, working_mount_directory)?;

    // after unpacking, the decompressed index is in <mount-dir>/base_index, move it to <mount-dir>
    std::fs::rename(
        format!("{}/base_index", working_mount_directory),
        index_mount_directory,
    )?;

    std::fs::remove_file(tarball_index_filename)?;

    Ok(())
}

async fn download_from_s3(
    client: &S3Client,
    config: &IKVStoreConfig,
    bucket: &str,
    key: &str,
    destination: &str,
) -> anyhow::Result<()> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(destination)?;

    let sse_key_objects = utils::encryption::sse_key_and_digest(config)?;
    let mut writer = BufWriter::new(&file);
    let mut result = client
        .get_object()
        // https://docs.aws.amazon.com/AmazonS3/latest/userguide/ServerSideEncryptionCustomerKeys.html
        .sse_customer_algorithm(aws_sdk_s3::types::ServerSideEncryption::Aes256.as_str())
        .sse_customer_key(sse_key_objects.0)
        .sse_customer_key_md5(sse_key_objects.1)
        .bucket(bucket)
        .key(key)
        .send()
        .await?;
    while let Some(bytes) = result.body.try_next().await? {
        writer.write_all(&bytes)?;
    }

    Ok(())
}

fn unpack_tarball(input_filepath: &str, destination_dir: &str) -> anyhow::Result<()> {
    // Unzip working_mount_dir/base_index.tar.gz to working_mount_dir/base_index
    // Reference: https://rust-lang-nursery.github.io/rust-cookbook/compression/tar.html
    let file = OpenOptions::new().read(true).open(input_filepath)?;
    let tar = GzDecoder::new(file);
    let mut archive = Archive::new(tar);
    archive.unpack(destination_dir)?;

    Ok(())
}

// input_dir: directory to tarball (ex. the index directory)
// output_filepath: of the the tarball file
fn pack_tarball(input_dir: &str, output_filepath: &str) -> anyhow::Result<()> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(output_filepath)?;
    let enc = GzEncoder::new(file, Compression::default());
    let mut tar = tar::Builder::new(enc);
    tar.append_dir_all("base_index", input_dir)?;
    Ok(())
}
