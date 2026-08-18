# my-smtp-sender

An http service which sends emails. The container carries its own MTA — [KumoMTA](https://kumomta.com)
(`kumod`, written in Rust) — so the mail is delivered straight to the mail server of the recipient:
no SendGrid, no smart host, no external relay.

```
   http POST /api/email/v1/send
              |
              v
   my-smtp-sender  --- builds the MIME message (lettre) --->  kumod  ---> MX of the recipient
   (this service)      submits it to 127.0.0.1:25             (queue, DKIM, retries, TLS)
```

The service is the entrypoint of the container. On start up it:

1. reads the settings model,
2. compiles the kumod configuration (policy + dkim keys) out of it,
3. starts `kumod` as a supervised child process and waits until it accepts the connections,
4. starts the http server.

A background timer restarts `kumod` if it ever dies, re-applying the current settings.

It is an infrastructure service, not a product one: no Seq, no telemetry, no metrics, no
service-sdk. Just `my-http-server`, `my-logger` and the settings model - the same stack
`my-no-sql-server` runs on. Everything it has to say goes to the standard output of the
container.

## Settings

The settings are read from `~/.my-smtp-sender` or from the `SETTINGS_URL` environment variable
(the standard MyJetTools settings model, reloaded in the background every 60 seconds).

```yaml
smtp:
  # Name the mail server introduces itself with. It MUST have an A record and a matching PTR
  # record of the outgoing ip address, otherwise the recipients reject the mail.
  my_hostname: mail.mydomain.com
  default_from_email: no-reply@mydomain.com
  default_from_name: My Service
  message_size_limit_mb: 25
  # How long the mail server keeps retrying before it gives up.
  max_queue_lifetime_hours: 24
  # Optional. Without it the mail is delivered to the mail server of the recipient
  # directly - which requires a public ip address with a matching PTR record and an
  # outgoing port 25 which is not blocked by the provider. When the relay is set, the
  # mail is handed over to it instead and it does the delivery.
  relay:
    host: smtp.mailgun.org
    port: 587
    user: postmaster@mydomain.com
    password: ...

# Optional. When the list is empty - the mail is sent unsigned.
dkim:
  - domain: mydomain.com
    selector: mail
    # Path of the file with the private key, not the key itself. `~` and the
    # environment variables are resolved.
    private_key_path: /etc/my-smtp-sender/dkim/mydomain.com.key
```

The key file itself is mounted into the container and stays out of the settings. On start up
the service copies it into the own directory of the mail server (`/opt/kumomta/etc/dkim/...`)
and gives it to the `kumod` user - the mounted file is neither modified nor chowned, because
on a bind mount that would change the owner of the file on the host as well.

A missing or empty key file stops the start up with the path in the error message, rather
than sending the mail unsigned.

The public part of each key has to be published as a TXT record of
`{selector}._domainkey.{domain}`.

## Building the image

The release pipeline builds the binary on the github runner and the `Dockerfile` only copies it
in. To build the image on a machine by hand - including a machine with no rust toolchain, or one
which is not linux - use the self contained one:

```bash
docker build -f Dockerfile.local -t my-smtp-sender .
```

## Running it

```yaml
services:
  my-smtp-sender:
    image: ghcr.io/my-jet-tools/my-smtp-sender:0.1.0
    restart: always
    ports:
      - "8000:8000"
    volumes:
      # The settings model.
      - ~/.my-smtp-sender:/root/.my-smtp-sender:ro
      # The dkim private keys, at the paths the settings point at.
      - ./dkim:/etc/my-smtp-sender/dkim:ro
      # The queue of the mail server. It MUST survive the restart of the container -
      # otherwise the mail which is accepted but not delivered yet is lost.
      - ./spool:/var/spool/kumomta
    # kumod sizes its memory budget from the cgroup limit of the container - give it one,
    # otherwise it takes 75% of the RAM of the whole host as its reference point.
    mem_limit: 1g
```

The smtp port is not published: it is bound to the loopback interface inside the container
and the only client of it is the service itself. The container has to be able to reach the
port 25 of the whole internet outbound, and to resolve dns.

## Api

Swagger is available at `/swagger`.

### POST /api/email/v1/send

```json
{
  "from_email": "no-reply@mydomain.com",
  "from_name": "My Service",
  "to": ["user@example.com"],
  "cc": [],
  "bcc": [],
  "subject": "Hello",
  "body": "<b>Hello</b>",
  "is_html": true,
  "attachments": [
    {
      "file_name": "invoice.pdf",
      "content_type": "application/pdf",
      "base64_content": "JVBERi0xLjQK..."
    }
  ]
}
```

`from_email`, `from_name`, `cc`, `bcc`, `is_html` and `attachments` are optional. When `from_email`
is not set — `smtp.default_from_email` from the settings is used. Both `user@domain.com` and
`User Name <user@domain.com>` are accepted in every address field.

The answer means the mail server has **accepted** the message and queued it — the delivery to the
recipient happens afterwards and is visible in the container log:

```json
{ "queue_id": "4bTgZ12Rz3zP", "smtp_response": "2.0.0 Ok: queued as 4bTgZ12Rz3zP" }
```

### GET /api/mail-server/v1/status

Tells whether the mail server process is alive and returns the summary of its queues.

### GET /api/isalive

Cheap health check - the name, the version and whether the mail server process is alive.
This is the one to point a docker health check at; the status endpoint above asks the mail
server itself and costs more.

## MCP

The same service is an MCP server on `/mcp` — the endpoint to register is
`http://{host}:8000/mcp`:

```bash
claude mcp add --transport http my-smtp-sender http://{host}:8000/mcp
```

| Tool | What it is for |
|---|---|
| `send_email` | Sends an email. The same flow the rest api uses. |
| `get_mail_server_status` | Is the mail server alive, how it signs and delivers, what is in the queues. |
| `get_mail_server_output` | Last lines of the stdout/stderr of the mail server — where the reason of a failed start up or of a deferred delivery is. |
| `get_mail_server_policy` | The configuration compiled out of the settings, as the mail server got it. |
| `restart_mail_server` | Applies the changed settings without redeploying the container. |
| `check_outbound_smtp` | Opens a tcp connection to a mail server and reads its greeting — the way to tell whether the provider blocks the outgoing port 25. |

Neither the rest api nor the MCP endpoint has any authentication: the service is meant to sit
in a private network. Do not expose the port 8000 to the internet.

## What decides whether the mail lands in the inbox

The service can only do half of it. The other half is dns and has to be set up once per domain:

* **PTR** of the outgoing ip address → `smtp.my_hostname`, and that name has to resolve back
  to the same address,
* **SPF** `TXT` record of the sending domain, allowing that ip address,
* **DKIM** `TXT` record of `{selector}._domainkey.{domain}`,
* **DMARC** `TXT` record of `_dmarc.{domain}`,
* the outgoing port 25 must not be blocked by the provider.

**Direct delivery needs all of it.** Gmail rejects the mail from an address without a PTR record
outright (`550 5.7.25`), Microsoft and Yahoo have the same rule since 2025, and a consumer
broadband address is treated with suspicion even when the PTR is in place. On a connection where
this can not be arranged - a home line, a provider which does not delegate the reverse dns, a
provider which blocks the port 25 - set `smtp.relay` and let a mail service do the last hop. The
rest of the service does not change: the api, the queue, the retries and the dkim signature stay
where they are.
