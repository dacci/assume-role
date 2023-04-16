## Usage

```console
$ assume-role --help
Usage: assume-role [OPTIONS] --role <NAME> [COMMAND]...

Arguments:
  [COMMAND]...  A command and its arguments to run as the assumed role. Runs current shell if not specified

Options:
  -r, --role <NAME>
          The name or the Amazon Resource Name (ARN) of the role to assume
      --role-session-name <NAME>
          An identifier for the assumed role session
      --policy-arn <ARN>
          The Amazon Resource Names (ARNs) of the IAM managed policy that you want to use as managed session policies
  -p, --policy <PATH>
          An IAM policy in JSON or YAML that you want to use as an inline session policy
      --duration-seconds <NUMBER>
          The duration, in seconds, of the role session
      --tag <KEY=VALUE>
          A session tag that you want to pass
      --transitive-tag-key <KEY>
          A key for session tags that you want to set as transitive
      --external-id <EXTERNAL_ID>
          A unique identifier that might be required when you assume a role in another account
      --serial-number <SERIAL_NUMBER>
          The identification number of the MFA device that is associated with the user who is making the `AssumeRole` call
      --token-code <TOKEN_CODE>
          The value provided by the MFA device, if the trust policy of the role being assumed requires MFA
      --source-identity <SOURCE_IDENTITY>
          The source identity specified by the principal that is calling the `AssumeRole` operation
  -h, --help
          Print help information
```

## Example

```console
$ aws sts get-caller-identity
{
    "UserId": "AIDACKCEVSQ6C2EXAMPLE",
    "Account": "111122223333",
    "Arn": "arn:aws:iam::111122223333:user/dacci"
}
$ assume-role -r AdministratorAccess --serial-number arn:aws:iam::111122223333:mfa/dacci --token-code 123456
Credentials will expire at 2023-04-16T10:47:01Z
$ aws sts get-caller-identity
{
    "UserId": "AROADBQP57FF2AEXAMPLE:assume-role@1681638421",
    "Account": "111122223333",
    "Arn": "arn:aws:sts::111122223333:assumed-role/AdministratorAccess/assume-role@1681638421"
}
```
