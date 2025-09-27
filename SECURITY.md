# Security Policy

## Supported Versions

| Version | Supported |
| ------- | --------- |
| 0.1.x   | ✅ |

The Trendmarket collector distribution is currently pre-release software. All
patches are evaluated on a rolling basis; when a new minor release is cut, the
previous one immediately enters security-maintenance mode.

## Reporting a Vulnerability

* Email: [security@creditengine.com](mailto:security@creditengine.com)
* Response target: acknowledgement within 2 business days, mitigation plan in 7
  business days.
* Please include reproduction steps, impacted components, and any suggested
  compensating controls. Encrypt sensitive disclosures with the PGP key
  published at <https://security.creditengine.com/pgp.txt>.

## Disclosure Process

1. We triage and reproduce the report.
2. If the issue is valid, we assign a severity using CVSS 4.0 and coordinate a
   fix with the owning team listed in [`CODEOWNERS`](CODEOWNERS).
3. When a fix is available, we communicate release timelines with the reporter
   and any downstream consumers registered in our security advisories mailing
   list.
4. A security advisory is published after rollout with mitigation details and
   CVE assignment (when applicable).

We appreciate coordinated disclosure and do not pursue legal action for
research performed in good faith within these guidelines.
