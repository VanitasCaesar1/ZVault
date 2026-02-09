//! Dashboard page content constants for the `VaultRS` web UI.
//!
//! Each public constant provides the inner HTML content for a specific app page.
//! The shared app shell (sidebar, topbar, CSS) is handled by [`super::ui::app_shell`].
//! Themed with a warm golden "treasure chest" aesthetic.

/// Script to fetch seal status and highlight the active sidebar link.
pub const SIDEBAR_SCRIPT: &str = r##"
<script>
(function(){
  var path=location.pathname;
  document.querySelectorAll('.sidebar-link').forEach(function(el){
    if(el.getAttribute('href')===path) el.classList.add('active');
  });
  fetch('/v1/sys/seal-status')
    .then(function(r){return r.json()})
    .then(function(d){
      var dot=document.getElementById('seal-dot');
      var txt=document.getElementById('seal-text');
      if(!d.initialized){
        dot.className='status-dot uninitialized';
        txt.textContent='Not Initialized';
      } else if(d.sealed){
        dot.className='status-dot sealed';
        txt.textContent='Sealed';
      } else {
        dot.className='status-dot unsealed';
        txt.textContent='Unsealed';
      }
    })
    .catch(function(){});
})();
</script>
"##;

/// Dashboard overview content with stat cards and recent activity.
pub const DASHBOARD_CONTENT: &str = r##"
<div class="stat-grid">
  <div class="stat-card">
    <div class="stat-card-label">Seal Status</div>
    <div class="stat-card-value danger" id="dash-seal">Sealed</div>
    <div class="stat-card-sub">Requires unseal shares to operate</div>
  </div>
  <div class="stat-card">
    <div class="stat-card-label">Active Secrets</div>
    <div class="stat-card-value primary">&mdash;</div>
    <div class="stat-card-sub">Across all mounted engines</div>
  </div>
  <div class="stat-card">
    <div class="stat-card-label">Active Leases</div>
    <div class="stat-card-value accent">&mdash;</div>
    <div class="stat-card-sub">Dynamic credentials outstanding</div>
  </div>
  <div class="stat-card">
    <div class="stat-card-label">Active Tokens</div>
    <div class="stat-card-value success">&mdash;</div>
    <div class="stat-card-sub">Authenticated sessions</div>
  </div>
</div>

<div style="display:grid;grid-template-columns:1fr 1fr;gap:20px">
  <div class="card">
    <div class="card-header">
      <span class="card-title">Mounted Engines</span>
    </div>
    <table class="table">
      <thead><tr><th>Path</th><th>Type</th><th>Status</th></tr></thead>
      <tbody>
        <tr><td><code>secret/</code></td><td>KV v2</td><td><span class="badge badge-success">Active</span></td></tr>
        <tr><td><code>transit/</code></td><td>Transit</td><td><span class="badge badge-success">Active</span></td></tr>
        <tr><td><code>pki/</code></td><td>PKI</td><td><span class="badge badge-muted">Planned</span></td></tr>
        <tr><td><code>database/</code></td><td>Database</td><td><span class="badge badge-muted">Planned</span></td></tr>
      </tbody>
    </table>
  </div>

  <div class="card">
    <div class="card-header">
      <span class="card-title">Recent Audit Events</span>
      <a href="/app/audit" class="btn btn-sm btn-secondary">View All</a>
    </div>
    <table class="table">
      <thead><tr><th>Time</th><th>Operation</th><th>Path</th></tr></thead>
      <tbody>
        <tr><td>2 min ago</td><td>read</td><td><code>secret/data/prod/db</code></td></tr>
        <tr><td>5 min ago</td><td>write</td><td><code>secret/data/prod/api-key</code></td></tr>
        <tr><td>12 min ago</td><td>login</td><td><code>auth/token/create</code></td></tr>
        <tr><td>18 min ago</td><td>encrypt</td><td><code>transit/encrypt/app-key</code></td></tr>
      </tbody>
    </table>
  </div>
</div>
"##;

/// Vault initialization wizard content.
pub const INIT_CONTENT: &str = r##"
<div class="wizard">
  <div class="wizard-step">
    <div class="wizard-step-num">1</div>
    <h3>Initialize Your Vault</h3>
    <p>Generate the root encryption key and split the unseal key into Shamir shares.
       This can only be done once. Store the shares securely &mdash; they cannot be recovered.</p>
    <div class="form-group">
      <label class="form-label">Number of Key Shares</label>
      <input type="number" class="form-input" id="init-shares" value="5" min="2" max="10"/>
      <div class="form-hint">Total unseal key shares to generate (2&ndash;10)</div>
    </div>
    <div class="form-group">
      <label class="form-label">Key Threshold</label>
      <input type="number" class="form-input" id="init-threshold" value="3" min="2" max="10"/>
      <div class="form-hint">Minimum shares required to unseal (2 to share count)</div>
    </div>
    <button class="btn btn-primary" id="init-btn" onclick="initVault()">Initialize Vault</button>
  </div>

  <div class="wizard-step" id="init-result" style="display:none">
    <div class="wizard-step-num">2</div>
    <h3>Save Your Unseal Shares</h3>
    <p>These shares are shown <strong>once</strong> and never stored by VaultRS.
       Distribute them to trusted operators. You need the threshold number of shares to unseal.</p>
    <div id="init-shares-list"></div>
    <div style="margin-top:20px">
      <label class="form-label">Root Token</label>
      <div class="code-block" id="init-root-token"></div>
      <div class="form-hint">Use this token for initial authentication. Create scoped tokens and revoke this one.</div>
    </div>
    <div style="margin-top:20px">
      <a href="/app/unseal" class="btn btn-primary">Proceed to Unseal</a>
    </div>
  </div>
</div>

<script>
function initVault(){
  var shares=document.getElementById('init-shares').value;
  var threshold=document.getElementById('init-threshold').value;
  var btn=document.getElementById('init-btn');
  btn.disabled=true;
  btn.textContent='Initializing...';
  fetch('/v1/sys/init',{
    method:'POST',
    headers:{'Content-Type':'application/json'},
    body:JSON.stringify({shares:parseInt(shares),threshold:parseInt(threshold)})
  })
  .then(function(r){return r.json()})
  .then(function(d){
    if(d.error){
      btn.disabled=false;
      btn.textContent='Initialize Vault';
      alert('Error: '+d.message);
      return;
    }
    var list=document.getElementById('init-shares-list');
    var html='';
    d.unseal_shares.forEach(function(s,i){
      html+='<div class="code-block"><span class="accent">Share '+(i+1)+':</span> <span class="key">'+s+'</span></div>';
    });
    list.innerHTML=html;
    document.getElementById('init-root-token').textContent=d.root_token;
    document.getElementById('init-result').style.display='block';
    btn.style.display='none';
  })
  .catch(function(e){
    btn.disabled=false;
    btn.textContent='Initialize Vault';
    alert('Network error: '+e.message);
  });
}
</script>
"##;

/// Unseal page content.
pub const UNSEAL_CONTENT: &str = r##"
<div class="wizard">
  <div class="wizard-step">
    <div class="wizard-step-num">&#x1f511;</div>
    <h3>Submit Unseal Share</h3>
    <p>Enter unseal key shares one at a time. When the threshold is reached,
       the vault will unseal and begin serving requests.</p>
    <div class="form-group">
      <label class="form-label">Unseal Key Share</label>
      <input type="text" class="form-input mono" id="unseal-share"
             placeholder="Paste a base64-encoded unseal share..."/>
    </div>
    <button class="btn btn-primary" id="unseal-btn" onclick="submitShare()">Submit Share</button>
    <div id="unseal-progress" style="margin-top:20px"></div>
  </div>

  <div class="wizard-step" id="unseal-success" style="display:none">
    <div class="wizard-step-num" style="background:#6B8E4E">&#x2713;</div>
    <h3>Vault Unsealed</h3>
    <p>The vault is now unsealed and ready to serve requests. All secrets engines are active.</p>
    <a href="/app" class="btn btn-primary">Go to Dashboard</a>
  </div>
</div>

<script>
function submitShare(){
  var share=document.getElementById('unseal-share').value;
  var btn=document.getElementById('unseal-btn');
  btn.disabled=true;
  btn.textContent='Submitting...';
  fetch('/v1/sys/unseal',{
    method:'POST',
    headers:{'Content-Type':'application/json'},
    body:JSON.stringify({share:share})
  })
  .then(function(r){return r.json()})
  .then(function(d){
    btn.disabled=false;
    btn.textContent='Submit Share';
    document.getElementById('unseal-share').value='';
    if(d.error){
      alert('Error: '+d.message);
      return;
    }
    if(!d.sealed){
      document.getElementById('unseal-success').style.display='block';
      var dot=document.getElementById('seal-dot');
      var txt=document.getElementById('seal-text');
      if(dot){dot.className='status-dot unsealed';}
      if(txt){txt.textContent='Unsealed';}
    } else {
      document.getElementById('unseal-progress').innerHTML=
        '<div class="badge badge-warning">Progress: '+d.progress+' / '+d.threshold+' shares submitted</div>';
    }
  })
  .catch(function(e){
    btn.disabled=false;
    btn.textContent='Submit Share';
    alert('Network error: '+e.message);
  });
}
</script>
"##;

/// Secrets browser content.
pub const SECRETS_CONTENT: &str = r##"
<div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:24px">
  <div>
    <div style="font-size:14px;color:var(--text-muted)">Browse and manage secrets across all mounted KV engines.</div>
  </div>
  <button class="btn btn-primary btn-sm">+ New Secret</button>
</div>

<div class="card">
  <div class="card-header">
    <span class="card-title">secret/ (KV v2)</span>
    <div style="display:flex;gap:8px">
      <input type="text" class="form-input" style="width:240px;padding:6px 12px;font-size:13px" placeholder="Search secrets..."/>
    </div>
  </div>
  <table class="table">
    <thead><tr><th>Path</th><th>Version</th><th>Last Modified</th><th>Actions</th></tr></thead>
    <tbody>
      <tr>
        <td><code>production/db-password</code></td>
        <td><span class="badge badge-primary">v3</span></td>
        <td>2 hours ago</td>
        <td><button class="btn btn-sm btn-secondary">View</button></td>
      </tr>
      <tr>
        <td><code>production/api-key</code></td>
        <td><span class="badge badge-primary">v1</span></td>
        <td>1 day ago</td>
        <td><button class="btn btn-sm btn-secondary">View</button></td>
      </tr>
      <tr>
        <td><code>staging/db-password</code></td>
        <td><span class="badge badge-primary">v2</span></td>
        <td>3 days ago</td>
        <td><button class="btn btn-sm btn-secondary">View</button></td>
      </tr>
      <tr>
        <td><code>production/stripe-key</code></td>
        <td><span class="badge badge-primary">v5</span></td>
        <td>1 week ago</td>
        <td><button class="btn btn-sm btn-secondary">View</button></td>
      </tr>
    </tbody>
  </table>
</div>
"##;

/// Policies page content.
pub const POLICIES_CONTENT: &str = r##"
<div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:24px">
  <div>
    <div style="font-size:14px;color:var(--text-muted)">Define path-based access rules for tokens and auth methods.</div>
  </div>
  <button class="btn btn-primary btn-sm">+ New Policy</button>
</div>

<div class="card">
  <table class="table">
    <thead><tr><th>Name</th><th>Rules</th><th>Type</th><th>Actions</th></tr></thead>
    <tbody>
      <tr>
        <td><strong>root</strong></td>
        <td>1 rule</td>
        <td><span class="badge badge-warning">Built-in</span></td>
        <td><button class="btn btn-sm btn-secondary">View</button></td>
      </tr>
      <tr>
        <td><strong>default</strong></td>
        <td>2 rules</td>
        <td><span class="badge badge-warning">Built-in</span></td>
        <td><button class="btn btn-sm btn-secondary">View</button></td>
      </tr>
      <tr>
        <td><strong>app-readonly</strong></td>
        <td>4 rules</td>
        <td><span class="badge badge-primary">Custom</span></td>
        <td>
          <button class="btn btn-sm btn-secondary">Edit</button>
          <button class="btn btn-sm btn-danger" style="margin-left:4px">Delete</button>
        </td>
      </tr>
      <tr>
        <td><strong>deploy-bot</strong></td>
        <td>3 rules</td>
        <td><span class="badge badge-primary">Custom</span></td>
        <td>
          <button class="btn btn-sm btn-secondary">Edit</button>
          <button class="btn btn-sm btn-danger" style="margin-left:4px">Delete</button>
        </td>
      </tr>
    </tbody>
  </table>
</div>
"##;

/// Audit log page content.
pub const AUDIT_CONTENT: &str = r##"
<div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:24px">
  <div>
    <div style="font-size:14px;color:var(--text-muted)">Immutable record of every operation. Sensitive fields are HMAC'd.</div>
  </div>
  <div style="display:flex;gap:8px">
    <input type="date" class="form-input" style="width:160px;padding:6px 12px;font-size:13px"/>
    <input type="text" class="form-input" style="width:200px;padding:6px 12px;font-size:13px" placeholder="Filter by path..."/>
  </div>
</div>

<div class="card">
  <table class="table">
    <thead><tr><th>Timestamp</th><th>Operation</th><th>Path</th><th>Actor</th><th>Status</th></tr></thead>
    <tbody>
      <tr>
        <td style="font-family:var(--mono);font-size:12px">2026-02-09 10:32:15</td>
        <td>read</td>
        <td><code>secret/data/prod/db</code></td>
        <td><code>hmac:a3f2...c891</code></td>
        <td><span class="badge badge-success">200</span></td>
      </tr>
      <tr>
        <td style="font-family:var(--mono);font-size:12px">2026-02-09 10:30:02</td>
        <td>write</td>
        <td><code>secret/data/prod/api-key</code></td>
        <td><code>hmac:b7e1...d452</code></td>
        <td><span class="badge badge-success">200</span></td>
      </tr>
      <tr>
        <td style="font-family:var(--mono);font-size:12px">2026-02-09 10:28:44</td>
        <td>encrypt</td>
        <td><code>transit/encrypt/app-key</code></td>
        <td><code>hmac:c4d9...e123</code></td>
        <td><span class="badge badge-success">200</span></td>
      </tr>
      <tr>
        <td style="font-family:var(--mono);font-size:12px">2026-02-09 10:25:11</td>
        <td>login</td>
        <td><code>auth/token/create</code></td>
        <td><code>hmac:d8f3...a567</code></td>
        <td><span class="badge badge-success">200</span></td>
      </tr>
      <tr>
        <td style="font-family:var(--mono);font-size:12px">2026-02-09 10:20:33</td>
        <td>read</td>
        <td><code>secret/data/staging/db</code></td>
        <td><code>hmac:e2a1...b789</code></td>
        <td><span class="badge badge-danger">403</span></td>
      </tr>
      <tr>
        <td style="font-family:var(--mono);font-size:12px">2026-02-09 10:18:07</td>
        <td>issue</td>
        <td><code>pki/issue/web-server</code></td>
        <td><code>hmac:f1b2...c345</code></td>
        <td><span class="badge badge-success">200</span></td>
      </tr>
    </tbody>
  </table>
</div>
"##;

/// Leases page content.
pub const LEASES_CONTENT: &str = r##"
<div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:24px">
  <div>
    <div style="font-size:14px;color:var(--text-muted)">Active leases for dynamic credentials. Expired leases are automatically revoked.</div>
  </div>
</div>

<div class="card">
  <table class="table">
    <thead><tr><th>Lease ID</th><th>Engine</th><th>Issued</th><th>TTL</th><th>Status</th><th>Actions</th></tr></thead>
    <tbody>
      <tr>
        <td><code style="font-size:12px">db/creds/readonly/a1b2c3</code></td>
        <td>database</td>
        <td>10 min ago</td>
        <td>1h</td>
        <td><span class="badge badge-success">Active</span></td>
        <td>
          <button class="btn btn-sm btn-secondary">Renew</button>
          <button class="btn btn-sm btn-danger" style="margin-left:4px">Revoke</button>
        </td>
      </tr>
      <tr>
        <td><code style="font-size:12px">db/creds/readwrite/d4e5f6</code></td>
        <td>database</td>
        <td>25 min ago</td>
        <td>2h</td>
        <td><span class="badge badge-success">Active</span></td>
        <td>
          <button class="btn btn-sm btn-secondary">Renew</button>
          <button class="btn btn-sm btn-danger" style="margin-left:4px">Revoke</button>
        </td>
      </tr>
      <tr>
        <td><code style="font-size:12px">pki/issue/web/g7h8i9</code></td>
        <td>pki</td>
        <td>2 hours ago</td>
        <td>720h</td>
        <td><span class="badge badge-success">Active</span></td>
        <td>
          <button class="btn btn-sm btn-secondary">Renew</button>
          <button class="btn btn-sm btn-danger" style="margin-left:4px">Revoke</button>
        </td>
      </tr>
      <tr>
        <td><code style="font-size:12px">db/creds/readonly/j0k1l2</code></td>
        <td>database</td>
        <td>3 hours ago</td>
        <td>1h</td>
        <td><span class="badge badge-danger">Expired</span></td>
        <td><span style="font-size:12px;color:var(--text-muted)">Auto-revoked</span></td>
      </tr>
    </tbody>
  </table>
</div>
"##;

/// Auth methods page content.
pub const AUTH_CONTENT: &str = r##"
<div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:24px">
  <div>
    <div style="font-size:14px;color:var(--text-muted)">Configure authentication methods for identity verification.</div>
  </div>
  <button class="btn btn-primary btn-sm">+ Enable Auth Method</button>
</div>

<div class="card">
  <table class="table">
    <thead><tr><th>Path</th><th>Type</th><th>Description</th><th>Status</th><th>Actions</th></tr></thead>
    <tbody>
      <tr>
        <td><code>token/</code></td>
        <td>Token</td>
        <td>Built-in token authentication</td>
        <td><span class="badge badge-success">Enabled</span></td>
        <td><button class="btn btn-sm btn-secondary">Configure</button></td>
      </tr>
      <tr>
        <td><code>approle/</code></td>
        <td>AppRole</td>
        <td>Machine-to-machine authentication</td>
        <td><span class="badge badge-muted">Planned</span></td>
        <td>
          <button class="btn btn-sm btn-primary">Enable</button>
        </td>
      </tr>
      <tr>
        <td><code>oidc/</code></td>
        <td>OIDC</td>
        <td>OpenID Connect via Spring identity</td>
        <td><span class="badge badge-muted">Planned</span></td>
        <td>
          <button class="btn btn-sm btn-primary">Enable</button>
        </td>
      </tr>
      <tr>
        <td><code>kubernetes/</code></td>
        <td>Kubernetes</td>
        <td>Service account authentication</td>
        <td><span class="badge badge-muted">Planned</span></td>
        <td>
          <button class="btn btn-sm btn-primary">Enable</button>
        </td>
      </tr>
    </tbody>
  </table>
</div>
"##;
