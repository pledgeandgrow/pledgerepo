# Organizations

## Creating and Managing Organizations

### Creating an Organization

1. Go to [npmjs.com](https://www.npmjs.com) > your avatar > "Add an Organization"
2. Enter organization name
3. Choose a plan (Free or Paid)
4. The org name becomes your scope: `@org-name`

### Converting Your User Account to an Organization

You can convert a user account to an organization:

1. Go to Account Settings
2. Click "Convert to Organization"
3. **Warning:** This is irreversible. You'll lose the ability to log in as that user.

### Requiring 2FA in Your Organization

1. Go to Organization Settings > Security
2. Enable "Require two-factor authentication"
3. All members must have 2FA enabled

### Renaming an Organization

1. Go to Organization Settings
2. Click "Rename Organization"
3. Enter the new name

**Note:** The old scope (`@old-name`) will still work for existing packages, but new packages must use the new scope.

### Deleting an Organization

1. Go to Organization Settings
2. Click "Delete Organization"
3. Confirm

**Warning:** All packages, teams, and members will be removed. Transfer packages first.

## Paying for Your Organization

### Upgrading to a Paid Organization Plan

Plans:
- **Free** — Unlimited public packages, no private packages
- **Teams** — Unlimited private packages with team collaboration

1. Go to Organization Settings > Billing
2. Click "Upgrade"
3. Choose a plan and enter payment info

### Viewing, Downloading, and Emailing Receipts

1. Go to Organization Settings > Billing
2. View billing history
3. Download or email receipts

### Updating Organization Billing Settings

- Change payment method
- Update billing address
- View current plan and renewal date

### Downgrading to a Free Organization Plan

1. Go to Organization Settings > Billing
2. Click "Downgrade"
3. Private packages become inaccessible (but not deleted)

## Managing Organization Members

### Adding Members

1. Go to Organization > Members
2. Click "Add Member"
3. Enter their npm username
4. Assign a role (Owner, Admin, Developer, Member)

### Accepting or Rejecting an Organization Invitation

When invited, you'll receive an email:
1. Click the invitation link
2. Log in to npm
3. Accept or reject

### Organization Roles and Permissions

| Role | Permissions |
|------|-------------|
| **Owner** | Full control: billing, members, teams, packages, delete org |
| **Admin** | Manage members, teams, packages (no billing) |
| **Developer** | Publish packages, manage teams (no member management) |
| **Member** | Read access to org packages |

### Managing Organization Permissions

- Assign roles when adding members
- Change roles in Organization > Members
- Remove members when they leave

### Removing Members

1. Go to Organization > Members
2. Find the member
3. Click "Remove"
4. Confirm

## Managing Teams

### About the Developers Team

Every organization has a default "developers" team. All members are automatically added to it.

### Creating Teams

1. Go to Organization > Teams
2. Click "Create Team"
3. Enter team name

### Adding Organization Members to Teams

1. Go to Organization > Teams > [Team Name]
2. Click "Add Member"
3. Select from organization members

### Removing Organization Members from Teams

1. Go to Organization > Teams > [Team Name]
2. Find the member
3. Click "Remove from Team"

### Managing Team Access to Organization Packages

Grant a team access to a package:

```bash
npm access grant read-write @org/package team-name
```

Or via website:
1. Go to Organization > Teams > [Team Name]
2. Click "Add Package"
3. Select the package and access level

Access levels:
- **Read-only** — Can download and view
- **Read-write** — Can download, view, and publish

### Removing Teams

1. Go to Organization > Teams > [Team Name]
2. Click "Delete Team"
3. Confirm

## Managing Organization Packages

### About Organization Scopes and Packages

Organization packages use the `@org-name/package-name` format:

```
@my-org/my-package
```

- **Public** — Visible to everyone, free
- **Private** — Restricted to org members and teams, requires paid plan

### Configuring Your npm Client with Your Organization Settings

```ini
# .npmrc
@my-org:registry=https://registry.npmjs.org
//registry.npmjs.org/:_authToken=your-token
```

Set default scope:

```bash
npm config set scope @my-org
```

### Creating and Publishing an Organization Scoped Package

```bash
# Create
npm init --scope=@my-org

# Publish public
npm publish --access public

# Publish private (default for scoped)
npm publish
```

Configure in `package.json`:

```json
{
  "publishConfig": {
    "access": "public"
  }
}
```
