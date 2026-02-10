//! Dashboard page content constants for the `ZVault` web UI.
//!
//! Each public constant provides the inner HTML content for a specific app page.
//! The shared app shell (sidebar, topbar, CSS) is handled by [`super::ui::app_shell`].
//! Themed with a warm amber Crextio-inspired glassmorphism aesthetic.

/// Script to fetch seal status and highlight the active sidebar link.
pub const SIDEBAR_SCRIPT: &str = r##"
<script>
(function(){
  var path=location.pathname;
  document.querySelectorAll('.sidebar-link').forEach(function(el){
    if(el.getAttribute('href')===path) el.classList.add('active');
  });
  function getCookie(n){var m=document.cookie.match(new RegExp('(?:^|; )'+n+'=([^;]*)'));return m?decodeURIComponent(m[1]):null}
  var token=getCookie('zvault-token');
  var headers={};
  if(token) headers['X-Vault-Token']=token;
  fetch('/v1/sys/seal-status',{headers:headers})
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

/// Dashboard overview — Crextio-style bento grid with mixed card sizes.
pub const DASHBOARD_CONTENT: &str = r##"
<style>
.bento{display:grid;grid-template-columns:repeat(4,1fr);grid-auto-rows:auto;gap:18px;margin-bottom:32px}
.bento-wide{grid-column:span 2}
.bento-tall{grid-row:span 2}
.bento-full{grid-column:span 4}
.progress-ring{position:relative;width:120px;height:120px;margin:0 auto}
.progress-ring svg{transform:rotate(-90deg)}
.progress-ring-text{position:absolute;top:50%;left:50%;transform:translate(-50%,-50%);font-size:28px;font-weight:800;color:var(--text)}
.mini-bar{display:flex;align-items:end;gap:4px;height:48px;margin-top:12px}
.mini-bar-col{flex:1;border-radius:4px 4px 0 0;background:rgba(232,168,23,.2);transition:all .2s;min-width:6px}
.mini-bar-col.active{background:linear-gradient(to top,#E8A817,#F5C842)}
.pill-tabs{display:flex;gap:6px;margin-bottom:20px}
.pill-tab{padding:6px 16px;border-radius:50px;font-size:12px;font-weight:600;background:rgba(0,0,0,.04);color:var(--text-muted);border:none;cursor:pointer;font-family:var(--font);transition:all .15s}
.pill-tab.active{background:var(--primary);color:#2D1F0E}
.activity-item{display:flex;align-items:center;gap:14px;padding:12px 0;border-bottom:1px solid rgba(255,255,255,.04)}
.activity-item:last-child{border-bottom:none}
.activity-dot{width:10px;height:10px;border-radius:50%;flex-shrink:0}
.activity-dot.read{background:#4CAF50}.activity-dot.write{background:#F5C842}.activity-dot.login{background:#5B9BD5}.activity-dot.encrypt{background:#AB47BC}.activity-dot.denied{background:#E74C3C}
.activity-text{flex:1;font-size:13px;color:#D4C4A8}
.activity-time{font-size:11px;color:#7A6543;white-space:nowrap}
.engine-row{display:flex;align-items:center;justify-content:space-between;padding:14px 0;border-bottom:1px solid var(--border-light)}
.engine-row:last-child{border-bottom:none}
.engine-info{display:flex;align-items:center;gap:12px}
.engine-icon{width:36px;height:36px;border-radius:10px;display:flex;align-items:center;justify-content:center;background:var(--primary-glow)}
.engine-icon svg{width:18px;height:18px;stroke:var(--primary)}
.engine-name{font-weight:700;font-size:14px}
.engine-path{font-size:12px;color:var(--text-muted);font-family:var(--mono)}
@media(max-width:1100px){.bento{grid-template-columns:repeat(2,1fr)}.bento-full{grid-column:span 2}}
@media(max-width:700px){.bento{grid-template-columns:1fr}.bento-wide,.bento-full{grid-column:span 1}}
</style>

<div class="bento">
  <!-- Seal Status — large card -->
  <div class="stat-card">
    <div class="stat-card-label">Seal Status</div>
    <div class="stat-card-value danger" id="dash-seal">Sealed</div>
    <div class="stat-card-sub">Requires unseal shares to operate</div>
  </div>

  <!-- Active Secrets -->
  <div class="stat-card">
    <div class="stat-card-label">Active Secrets</div>
    <div class="stat-card-value primary">&mdash;</div>
    <div class="stat-card-sub">Across all mounted engines</div>
  </div>

  <!-- Active Leases -->
  <div class="stat-card">
    <div class="stat-card-label">Active Leases</div>
    <div class="stat-card-value accent">&mdash;</div>
    <div class="stat-card-sub">Dynamic credentials outstanding</div>
  </div>

  <!-- Active Tokens -->
  <div class="stat-card">
    <div class="stat-card-label">Active Tokens</div>
    <div class="stat-card-value success">&mdash;</div>
    <div class="stat-card-sub">Authenticated sessions</div>
  </div>

  <!-- Mounted Engines — wide glass card -->
  <div class="card bento-wide">
    <div class="card-header">
      <span class="card-title">Mounted Engines</span>
    </div>
    <div class="card-body" style="padding:16px 22px">
      <div class="engine-row">
        <div class="engine-info">
          <div class="engine-icon"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 2l-2 2m-7.61 7.61a5.5 5.5 0 11-7.778 7.778 5.5 5.5 0 017.777-7.777zm0 0L15.5 7.5m0 0l3 3L22 7l-3-3m-3.5 3.5L19 4"/></svg></div>
          <div><div class="engine-name">KV v2</div><div class="engine-path">secret/</div></div>
        </div>
        <span class="badge badge-success">Active</span>
      </div>
      <div class="engine-row">
        <div class="engine-info">
          <div class="engine-icon"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/></svg></div>
          <div><div class="engine-name">Transit</div><div class="engine-path">transit/</div></div>
        </div>
        <span class="badge badge-success">Active</span>
      </div>
      <div class="engine-row">
        <div class="engine-info">
          <div class="engine-icon" style="background:rgba(0,0,0,.04)"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="stroke:var(--text-light)"><circle cx="12" cy="12" r="10"/><path d="M12 6v6l4 2"/></svg></div>
          <div><div class="engine-name" style="color:var(--text-light)">PKI</div><div class="engine-path">pki/</div></div>
        </div>
        <span class="badge badge-muted">Planned</span>
      </div>
      <div class="engine-row">
        <div class="engine-info">
          <div class="engine-icon" style="background:rgba(0,0,0,.04)"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="stroke:var(--text-light)"><rect x="3" y="3" width="7" height="7" rx="1"/><rect x="14" y="3" width="7" height="7" rx="1"/><rect x="3" y="14" width="7" height="7" rx="1"/><rect x="14" y="14" width="7" height="7" rx="1"/></svg></div>
          <div><div class="engine-name" style="color:var(--text-light)">Database</div><div class="engine-path">database/</div></div>
        </div>
        <span class="badge badge-muted">Planned</span>
      </div>
    </div>
  </div>

  <!-- Recent Activity — dark card (Crextio onboarding panel style) -->
  <div class="card dark bento-wide">
    <div class="card-header">
      <span class="card-title">Recent Activity</span>
      <a href="/app/audit" class="btn btn-sm" style="background:rgba(245,200,66,.15);color:#F5C842;border-radius:50px;font-size:11px;padding:5px 14px">View All</a>
    </div>
    <div class="card-body" style="padding:8px 22px">
      <div class="activity-item"><span class="activity-dot read"></span><span class="activity-text"><code style="background:rgba(255,255,255,.06);color:#D4C4A8">secret/data/prod/db</code> &mdash; read</span><span class="activity-time">2 min ago</span></div>
      <div class="activity-item"><span class="activity-dot write"></span><span class="activity-text"><code style="background:rgba(255,255,255,.06);color:#D4C4A8">secret/data/prod/api-key</code> &mdash; write</span><span class="activity-time">5 min ago</span></div>
      <div class="activity-item"><span class="activity-dot login"></span><span class="activity-text"><code style="background:rgba(255,255,255,.06);color:#D4C4A8">auth/token/create</code> &mdash; login</span><span class="activity-time">12 min ago</span></div>
      <div class="activity-item"><span class="activity-dot encrypt"></span><span class="activity-text"><code style="background:rgba(255,255,255,.06);color:#D4C4A8">transit/encrypt/app-key</code> &mdash; encrypt</span><span class="activity-time">18 min ago</span></div>
      <div class="activity-item"><span class="activity-dot denied"></span><span class="activity-text"><code style="background:rgba(255,255,255,.06);color:#D4C4A8">secret/data/staging/db</code> &mdash; denied</span><span class="activity-time">20 min ago</span></div>
    </div>
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
    <p>These shares are shown <strong>once</strong> and never stored by ZVault.
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
    <div class="wizard-step-num" style="background:linear-gradient(135deg,#4CAF50,#66BB6A)">&#x2713;</div>
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
        '<div class="badge badge-warning" style="font-size:13px;padding:8px 18px">Progress: '+d.progress+' / '+d.threshold+' shares submitted</div>';
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
      <input type="text" class="form-input" style="width:240px;padding:7px 14px;font-size:13px;border-radius:50px" placeholder="Search secrets..."/>
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
    <input type="date" class="form-input" style="width:160px;padding:7px 14px;font-size:13px;border-radius:50px"/>
    <input type="text" class="form-input" style="width:200px;padding:7px 14px;font-size:13px;border-radius:50px" placeholder="Filter by path..."/>
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

/// Login page content — standalone card with token input.
pub const LOGIN_CONTENT: &str = r##"
<div style="width:100%;max-width:440px;padding:0 20px">
  <div style="text-align:center;margin-bottom:36px">
    <svg viewBox="0 0 32 32" fill="none" style="width:48px;height:48px;margin-bottom:16px"><defs><linearGradient id="zg" x1="0" y1="0" x2="32" y2="32"><stop offset="0%" stop-color="#F5C842"/><stop offset="100%" stop-color="#E8A817"/></linearGradient></defs><rect width="32" height="32" rx="8" fill="url(#zg)"/><path d="M9 11h14l-14 10h14" stroke="#2D1F0E" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"/></svg>
    <h1 style="font-size:28px;font-weight:800;color:var(--text);letter-spacing:-.5px;margin-bottom:6px">Sign in to ZVault</h1>
    <p style="font-size:14px;color:var(--text-muted)">Enter your vault token to access the dashboard.</p>
  </div>
  <div style="background:var(--glass);backdrop-filter:blur(16px);border:1px solid var(--glass-border);border-radius:var(--radius-lg);padding:32px;box-shadow:var(--glass-shadow)">
    <div class="form-group">
      <label class="form-label">Vault Token</label>
      <input type="password" class="form-input mono" id="login-token" placeholder="hvs.CAESIG..." autocomplete="off"/>
      <div class="form-hint">The root token from initialization, or a scoped token.</div>
    </div>
    <div id="login-error" style="display:none;background:var(--danger-light);color:#C62828;padding:10px 16px;border-radius:var(--radius-sm);font-size:13px;font-weight:600;margin-bottom:16px"></div>
    <button class="btn btn-primary" style="width:100%;padding:12px" id="login-btn" onclick="doLogin()">Sign In</button>
  </div>
  <div style="text-align:center;margin-top:20px;font-size:13px;color:var(--text-light)">
    Don't have a token? <a href="/app/init" style="color:var(--primary);font-weight:600;text-decoration:none">Initialize the vault</a>
  </div>
</div>

<script>
function doLogin(){
  var token=document.getElementById('login-token').value.trim();
  var btn=document.getElementById('login-btn');
  var errEl=document.getElementById('login-error');
  errEl.style.display='none';
  if(!token){errEl.textContent='Please enter a vault token.';errEl.style.display='block';return}
  btn.disabled=true;btn.textContent='Verifying...';
  fetch('/v1/auth/token/lookup-self',{method:'POST',headers:{'X-Vault-Token':token,'Content-Type':'application/json'},body:'{}'})
    .then(function(r){
      if(!r.ok) throw new Error('Invalid or expired token');
      return r.json();
    })
    .then(function(){
      document.cookie='zvault-token='+encodeURIComponent(token)+';path=/;max-age=86400;SameSite=Strict';
      window.location.href='/app';
    })
    .catch(function(e){
      btn.disabled=false;btn.textContent='Sign In';
      errEl.textContent=e.message||'Authentication failed';errEl.style.display='block';
    });
}
document.addEventListener('keydown',function(e){if(e.key==='Enter')doLogin()});
</script>
"##;
