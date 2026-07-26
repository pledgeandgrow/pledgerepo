# npm Policies

## Terms of Use

The npm Terms of Use govern the use of npm services, including:
- The npm registry
- The npm website
- The npm CLI

Key points:
- Users must be 13 or older
- Users are responsible for their accounts and content
- Packages must not contain malware or malicious code
- npm reserves the right to remove content that violates terms

Full text: [docs.npmjs.com/policies/terms](https://docs.npmjs.com/policies/terms)

## Open Source Terms

Specific terms for open source use of npm:
- Free for open source development
- Public packages are free to publish and download
- No warranty — packages are provided "as is"

Full text: [docs.npmjs.com/policies/open-source-terms](https://docs.npmjs.com/policies/open-source-terms)

## Private Terms

Specific terms for paid/private use:
- Private packages require a paid plan
- Organizations require a paid plan for private packages
- Data confidentiality for private packages
- Subscription billing terms

Full text: [docs.npmjs.com/policies/private-terms](https://docs.npmjs.com/policies/private-terms)

## Code of Conduct

The npm Code of Conduct applies to:
- npm website
- npm community forums
- npm GitHub repositories
- npm events

Key principles:
- Be respectful and inclusive
- No harassment or discrimination
- Report violations to conduct@npmjs.com

Full text: [docs.npmjs.com/policies/conduct](https://docs.npmjs.com/policies/conduct)

## Package Name Disputes

If you believe someone is squatting on a package name that belongs to you:

1. Contact the package owner directly first
2. If no response, file a dispute with npm support
3. Provide evidence of trademark or prior use

npm will mediate disputes following the [Package Name Disputes policy](https://docs.npmjs.com/policies/disputes).

## npm License

npm CLI is open source:
- npm CLI: MIT License
- npm registry: Proprietary (operated by GitHub/npm)
- Packages: Each package has its own license

Full text: [docs.npmjs.com/policies/npm-license](https://docs.npmjs.com/policies/npm-license)

## Privacy Policy

npm's Privacy Policy covers:
- Data collected: account info, usage data, package metadata
- How data is used: service operation, security, communication
- Data sharing: only as described or required by law
- User rights: access, export, deletion

Full text: [docs.npmjs.com/policies/privacy](https://docs.npmjs.com/policies/privacy)

## Unpublish Policy

npm's unpublish policy balances user freedom with ecosystem stability:

- **Within 72 hours**: Packages can be unpublished freely
- **After 72 hours**: Packages can only be unpublished if:
  - No other packages depend on it
  - It has been published for less than 72 hours
  - You contact npm support and get approval
- **Packages with dependents**: Generally cannot be unpublished

**Recommendation**: Use `npm deprecate` instead of unpublishing.

Full text: [docs.npmjs.com/policies/unpublish](https://docs.npmjs.com/policies/unpublish)

## Copyright and DMCA Policy

npm follows DMCA procedures for copyright infringement:

1. Submit a DMCA takedown notice
2. npm will remove the infringing content
3. The package owner can file a counter-notice
4. Content may be restored if counter-notice is filed

Contact: copyright@npmjs.com

Full text: [docs.npmjs.com/policies/dmca](https://docs.npmjs.com/policies/dmca)

## Logos and Usage

npm logo and brand usage guidelines:
- npm name and logo can be used for attribution
- Cannot imply official endorsement
- Cannot use npm branding for commercial products without permission

Full text: [docs.npmjs.com/policies/logos-and-usage](https://docs.npmjs.com/policies/logos-and-usage)

## Security Policy

npm's security policy covers:
- Reporting vulnerabilities in npm services
- Security disclosures and timelines
- Bug bounty program
- Security best practices for users

Report security issues: security@npmjs.com

Full text: [docs.npmjs.com/policies/security](https://docs.npmjs.com/policies/security)

## Replication and Web Crawler Policy

Rules for mirroring or crawling the npm registry:

- **Mirroring**: Contact npm for permission
- **Crawling**: Rate limits apply; respect `robots.txt`
- **Data dumps**: Available via the registry API
- **Commercial use**: Requires a license agreement

Full text: [docs.npmjs.com/policies/crawlers](https://docs.npmjs.com/policies/crawlers)

## Threats and Mitigations

npm documents known threats to the package ecosystem and their mitigations.

### Account Takeovers

#### By Compromising Passwords

This is the most common attack. Mitigations:

- **Enable 2FA** — The strongest option is a security key (built-in or hardware). Also supports authenticator apps.
- **Mandatory 2FA** — npm has rolled out mandatory 2FA for top-100 and top-500 package maintainers, and is extending to all high-impact packages (1M+ weekly downloads or 500+ dependents).
- **Enhanced login verification** — For users without 2FA, npm sends a one-time password over email to protect from account takeover.

#### By Registering an Expired Email Domain

Attackers identify accounts using expired domains for their email, register the domain, recreate the email, and take over the account via password reset.

Mitigations:
- npm periodically checks for expired domains and invalid MX records
- When a domain has expired, npm disables password reset and requires account recovery
- Email addresses in public package metadata are not updated when maintainers change their email, reducing the effectiveness of this attack

### Uploading Malicious Packages

#### By "Typosquatting" / Dependency Confusion

Attackers register packages with names similar to popular packages, hoping users will mistype.

- **Typosquatting** — npm can detect and block publishing of typosquat packages
- **Dependency confusion** — Attackers register public packages with the same name as private org packages. Mitigation: **use scoped packages** (`@org/name`) to ensure private packages aren't substituted from the public registry

#### By Changing an Existing Package to Have Malicious Behavior

Attackers add malicious code to existing popular packages.

- npm scans packages for known malicious content (in partnership with Microsoft)
- npm runs packages to detect new malicious behavior patterns
- The Trust and Safety team checks and removes reported malicious content
- Report suspected malware: [docs.npmjs.com/reporting-malware-in-an-npm-package](https://docs.npmjs.com/reporting-malware-in-an-npm-package)

### Supply Chain Attack Summary

| Threat | Mitigation |
|--------|------------|
| Password compromise | Enable 2FA (security key preferred) |
| Expired email domain | npm checks domains, disables password reset |
| Typosquatting | npm auto-detects and blocks |
| Dependency confusion | Use scoped packages (`@org/name`) |
| Malicious package updates | npm scans + runs packages for malicious behavior |
| Malicious install scripts | `--ignore-scripts`, `npm approve-scripts` |
| Compromised dependencies | Lock files, `npm ci`, dependency review |
| Account compromise | 2FA, granular access tokens with expiration |

### Best Practices Summary

- Enable 2FA on your account (security key preferred)
- Use granular access tokens with expiration
- Run `npm audit` regularly
- Use `npm ci` in CI/CD
- Verify package signatures
- Publish with provenance
- Review new dependencies before installing
- Use `--ignore-scripts` for untrusted packages
- Use scoped packages to prevent dependency confusion
- Keep your email domain registered and up to date
