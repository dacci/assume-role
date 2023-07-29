use anyhow::{anyhow, Context as _, Result};
use aws_sdk_sts::types::{PolicyDescriptorType, Tag};
use chrono::Utc;
use clap::Parser;
use tokio::fs::File;
use tokio::process::Command;

#[derive(clap::Parser)]
struct Args {
    /// The name or the Amazon Resource Name (ARN) of the role to assume.
    #[arg(short, long, value_name = "NAME")]
    role: String,

    /// An identifier for the assumed role session.
    #[arg(long, value_name = "NAME")]
    role_session_name: Option<String>,

    /// The Amazon Resource Names (ARNs) of the IAM managed policy that you want to use as managed session policies.
    #[arg(long, value_name = "ARN")]
    policy_arn: Vec<String>,

    /// An IAM policy in JSON or YAML that you want to use as an inline session policy.
    #[arg(short, long, value_name = "PATH")]
    policy: Option<String>,

    /// The duration, in seconds, of the role session.
    #[arg(long, value_name = "NUMBER")]
    duration_seconds: Option<i32>,

    /// A session tag that you want to pass.
    #[arg(long, value_name = "KEY=VALUE")]
    tag: Vec<String>,

    /// A key for session tags that you want to set as transitive.
    #[arg(long, value_name = "KEY")]
    transitive_tag_key: Vec<String>,

    /// A unique identifier that might be required when you assume a role in another account.
    #[arg(long)]
    external_id: Option<String>,

    /// The identification number of the MFA device that is associated with the user who is making the `AssumeRole` call.
    #[arg(long)]
    serial_number: Option<String>,

    /// The value provided by the MFA device, if the trust policy of the role being assumed requires MFA.
    #[arg(long)]
    token_code: Option<String>,

    /// The source identity specified by the principal that is calling the `AssumeRole` operation.
    #[arg(long)]
    source_identity: Option<String>,

    /// A command and its arguments to run as the assumed role. Runs current shell if not specified.
    command: Vec<String>,
}

fn main() -> Result<()> {
    use tracing_subscriber::prelude::*;

    let args: Args = Args::parse();

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::builder().from_env().unwrap())
        .init();

    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async_main(args))
}

async fn async_main(args: Args) -> Result<()> {
    let config = aws_config::load_from_env().await;
    let sts = aws_sdk_sts::Client::new(&config);

    let role_arn = if args.role.starts_with("arn:") {
        args.role
    } else {
        let iam = aws_sdk_iam::Client::new(&config);
        let response = iam.get_role().role_name(args.role).send().await?;
        response
            .role()
            .ok_or_else(|| anyhow!("role is not provided"))
            .and_then(|r| r.arn().ok_or_else(|| anyhow!("arn is not provided")))?
            .to_string()
    };

    let mut request = sts
        .assume_role()
        .role_arn(role_arn)
        .role_session_name(
            args.role_session_name
                .unwrap_or_else(|| format!("assume-role@{}", Utc::now().timestamp())),
        )
        .set_policy_arns(Some(
            args.policy_arn
                .iter()
                .map(|s| PolicyDescriptorType::builder().arn(s).build())
                .collect(),
        ))
        .set_duration_seconds(args.duration_seconds)
        .set_transitive_tag_keys(Some(args.transitive_tag_key))
        .set_external_id(args.external_id)
        .set_serial_number(args.serial_number)
        .set_token_code(args.token_code)
        .set_source_identity(args.source_identity);

    for tag in &args.tag {
        if let Some((key, value)) = tag.split_once('=') {
            request = request.tags(Tag::builder().key(key).value(value).build());
        } else {
            return Err(anyhow!("illegal tag: `{tag}`"));
        }
    }

    if let Some(path) = &args.policy {
        let f = File::open(path)
            .await
            .with_context(|| format!("failed to open `{path}`"))?
            .into_std()
            .await;
        let value: serde_yaml::Value =
            serde_yaml::from_reader(f).with_context(|| format!("failed to read `{path}`"))?;

        let policy = serde_json::to_string(&value).context("malformed policy")?;
        request = request.policy(policy);
    }

    let response = request.send().await?;

    let Some(credentials) = response.credentials() else {
        return Err(anyhow!("no credentials provided"));
    };

    let Some(access_key_id) = credentials.access_key_id()  else {
        return Err(anyhow!("no access_key_id provided"));
    };

    let Some(secret_access_key) = credentials.secret_access_key() else {
        return Err(anyhow!("no secret_access_key provided"));
    };

    if let Some(expiration) = credentials.expiration() {
        println!(
            "Credentials will expire at {}",
            expiration.fmt(aws_smithy_types::date_time::Format::DateTime)?
        );
    }

    let mut cmd = if args.command.is_empty() {
        Command::new(std::env::var("SHELL").context("failed to get environment variable `SHELL`")?)
    } else {
        let mut iter = args.command.iter();
        let mut cmd = Command::new(iter.next().unwrap());
        cmd.args(iter);
        cmd
    };

    cmd.env("AWS_ACCESS_KEY_ID", access_key_id)
        .env("AWS_SECRET_ACCESS_KEY", secret_access_key);
    if let Some(session_token) = credentials.session_token() {
        cmd.env("AWS_SESSION_TOKEN", session_token);
    }

    cmd.spawn()?.wait().await?;

    Ok(())
}
