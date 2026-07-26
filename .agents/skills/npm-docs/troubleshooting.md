# Troubleshooting

## Generating and Locating npm-debug.log Files

### Enabling Debug Logs

```bash
# Run with verbose logging
npm install --loglevel=verbose

# Or set debug environment
DEBUG=* npm install

# Generate a log file
npm install --logs-dir=./logs
```

### Finding Log Files

Logs are stored in:

```bash
# Default location
~/.npm/_logs/

# List recent logs
ls -lt ~/.npm/_logs/ | head
```

### Sharing Logs

When reporting issues, include:
- The debug log file
- Your npm version: `npm --version`
- Your Node.js version: `node --version`
- Your OS and architecture

## Common Errors

### EACCES: Permission Denied

**Error:**
```
npm ERR! Error: EACCES: permission denied, access '/usr/local/lib/node_modules'
```

**Solutions:**
1. Use a version manager (nvm, fnm) — recommended
2. Change npm's default directory: `npm config set prefix '~/.npm-global'`
3. Fix permissions: `sudo chown -R $(whoami) $(npm config get prefix)/{lib/node_modules,bin,share}`
4. Use `npx` instead of global install

See `installing-packages.md` for detailed solutions.

### ENOENT: No Such File or Directory

**Error:**
```
npm ERR! enoent ENOENT: no such file or directory, open 'package.json'
```

**Solution:**
```bash
npm init -y
```

### ERESOLVE: Could Not Resolve Dependency Tree

**Error:**
```
npm ERR! ERESOLVE unable to resolve dependency tree
```

**Solutions:**
```bash
# Use legacy peer deps handling
npm install --legacy-peer-deps

# Force install
npm install --force

# Clear cache and reinstall
npm cache clean --force
rm -rf node_modules package-lock.json
npm install
```

### ETIMEDOUT / ECONNREFUSED

**Error:**
```
npm ERR! network ETIMEDOUT
npm ERR! network ECONNREFUSED
```

**Solutions:**
```bash
# Check registry URL
npm config get registry

# Try a different registry
npm config set registry https://registry.npmmirror.com

# Check proxy settings
npm config get proxy
npm config get https-proxy

# Clear cache
npm cache clean --force

# Increase timeout
npm config set fetch-timeout 120000
```

### E401: Unauthorized

**Error:**
```
npm ERR! code E401
npm ERR! Unable to authenticate
```

**Solutions:**
```bash
# Login again
npm login

# Check your token
npm config get //registry.npmjs.org/:_authToken

# Update token
npm config set //registry.npmjs.org/:_authToken "your-new-token"
```

### E403: Forbidden

**Error:**
```
npm ERR! code E403
npm ERR! 403 Forbidden
```

**Causes:**
- Publishing a scoped package without `--access public`
- Insufficient permissions for the package
- 2FA required but not provided
- Account suspended

**Solutions:**
```bash
# For scoped public packages
npm publish --access public

# For 2FA
npm publish --otp=123456

# Check permissions
npm access list collaborators @scope/package
```

### E404: Not Found

**Error:**
```
npm ERR! code E404
npm ERR! 404 Not Found
```

**Causes:**
- Package name is misspelled
- Package is private and you lack access
- Package was unpublished
- Wrong registry configured

**Solutions:**
```bash
# Verify package exists
npm view <package-name>

# Check your registry
npm config get registry

# For scoped packages, check scope registry
npm config get @scope:registry
```

### EEXIST: File Already Exists

**Error:**
```
npm ERR! code EEXIST
npm ERR! EEXIST: file already exists
```

**Solution:**
```bash
npm cache clean --force
rm -rf node_modules package-lock.json
npm install
```

### Peer Dependency Conflicts

**Error:**
```
npm ERR! Conflicting peer dependency
```

**Solutions:**
```bash
# Ignore peer deps
npm install --legacy-peer-deps

# Or manually resolve by updating conflicting packages
npm update <conflicting-package>
```

### Module Not Found

**Error:**
```
Error: Cannot find module 'some-package'
```

**Solutions:**
```bash
# Reinstall
npm install

# Check if it's in package.json
npm ls some-package

# Check node_modules
ls node_modules/some-package

# Rebuild native modules
npm rebuild
```

### npm Command Not Found

**Solutions:**
```bash
# Check if npm is installed
which npm  # Unix
where npm  # Windows

# Reinstall Node.js
# Download from https://nodejs.org/

# Or use a version manager
nvm install --lts
```

### Stuck on "added X packages"

**Solutions:**
```bash
# Clear cache
npm cache clean --force

# Remove node_modules and reinstall
rm -rf node_modules
npm install

# Use --verbose to see what's happening
npm install --loglevel=verbose
```

## Try the Latest Stable Version of npm

```bash
# Update npm globally
npm install -g npm@latest

# Check version
npm --version

# If that fails, use npx
npx npm@latest install
```

## Try the Latest Stable Version of Node.js

```bash
# Using nvm
nvm install node
nvm use node

# Using fnm
fnm install latest
fnm use latest

# Download from nodejs.org
# https://nodejs.org/en/download/
```

## Debugging Tips

### Clean Reinstall

```bash
# Nuclear option
rm -rf node_modules package-lock.json ~/.npm/_cacache
npm install
```

### Verbose Output

```bash
npm install --loglevel=silly 2>&1 | tee npm-debug.log
```

### Check npm Config

```bash
# All config with sources
npm config list -l

# Check specific values
npm config get registry
npm config get prefix
npm config get cache
```

### Check Node.js Compatibility

```bash
# Check engines
node -e "console.log(process.versions)"

# Verify package engines
npm ls --depth=0
```
