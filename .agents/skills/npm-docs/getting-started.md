# Getting Started

## Setting Up Your npm User Account

### Creating a New User Account

1. Go to [npmjs.com/signup](https://www.npmjs.com/signup)
2. Enter your username, email, and password
3. Complete the verification email
4. Your account is available at `https://www.npmjs.com/~yourusername`

Or via CLI:

```bash
npm login
```

### Creating a Strong Password

npm passwords must:
- Be at least 8 characters long
- Contain at least one lowercase letter
- Contain at least one uppercase letter
- Contain at least one number
- Not be a commonly used password

### Receiving a One-Time Password Over Email

npm can send a one-time password (OTP) to your email for authentication when you don't have 2FA enabled.

## Two-Factor Authentication (2FA)

### About 2FA

Two-factor authentication adds an extra layer of security. npm supports:
- **TOTP** — Time-based one-time passwords (authenticator apps like Google Authenticator, Authy)
- **OTP over email** — One-time passwords sent to your email

### Configuring 2FA

1. Go to your account settings on npmjs.com
2. Select "Account Settings" > "Two-Factor Authentication"
3. Choose your preferred method (authenticator app or email)
4. Scan the QR code or enter the setup key
5. Verify with a 6-digit code

You can configure 2FA for:
- **Authorization only** — Required for login and sensitive actions
- **Authorization and writes** — Required for login, publishing, and settings changes

### Accessing npm Using 2FA

When 2FA is enabled, you'll need to provide an OTP for:
- Logging in to the CLI
- Publishing packages
- Changing account settings
- Deleting packages

```bash
npm publish --otp=123456
```

### Recovering Your 2FA-Enabled Account

If you lose access to your 2FA device:
1. Use backup codes provided during setup
2. Request account recovery via email
3. Contact npm support if backup codes are unavailable

## Managing Your npm User Account

### Managing Your Profile Settings

- Update your name, avatar, and bio
- Manage email addresses
- Configure notification preferences
- View your packages and organizations

### Changing Your npm Username

npm usernames **cannot** be changed once created. To use a different username, you must:
1. Create a new account
2. Transfer your packages to the new account
3. Delete the old account

### Deleting Your npm User Account

1. Go to Account Settings
2. Click "Delete Account"
3. Confirm with your password

**Warning:** This is permanent. Transfer or unpublish your packages first.

### Requesting an Export of Your Personal Data

You can request a data export under GDPR/CCPA:
1. Go to Account Settings
2. Click "Request Data Export"
3. npm will email you a link to download your data

## Paying for Your npm User Account

### Upgrading to a Paid Plan

Paid plans enable:
- Unlimited private packages
- Private packages with team access
- Organization creation

Plans:
- **Free** — Unlimited public packages, no private packages
- **Pro** — Unlimited private packages for personal use
- **Teams** — Private packages with team collaboration

### Viewing, Downloading, and Emailing Receipts

1. Go to Account Settings > Billing
2. View your billing history
3. Download or email individual receipts

### Updating Billing Settings

- Change payment method (credit card)
- Update billing address
- View current plan and renewal date

### Downgrading to a Free Plan

1. Go to Account Settings > Billing
2. Click "Downgrade"
3. Your private packages will become inaccessible (but not deleted)

## Configuring Your Local Environment

### About npm CLI Versions

npm is versioned independently from Node.js. The CLI follows semver. Check your version:

```bash
npm --version
```

### Downloading and Installing Node.js and npm

**Option 1: Official Installer**
- Download from [nodejs.org](https://nodejs.org/)
- Includes npm

**Option 2: Version Manager (recommended)**
```bash
# Using nvm (Unix/macOS)
nvm install --lts
nvm use --lts

# Using nvm-windows (Windows)
nvm install lts
nvm use lts

# Using fnm
fnm install --lts
fnm use --lts
```

**Option 3: Direct npm install**
```bash
npm install -g npm@latest
```

### Try the Latest Stable Version of npm

```bash
npm install -g npm@latest
# Or a specific version
npm install -g npm@10.0.0
```

### Try the Latest Stable Version of Node.js

Use a version manager to switch between Node.js versions:

```bash
nvm install node        # Latest
nvm install --lts       # LTS
nvm use node
nvm use --lts
```
