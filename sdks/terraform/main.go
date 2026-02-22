// ZVault Terraform Provider
//
// This is the entry point for the Terraform provider plugin.
// Build with: go build -o terraform-provider-zvault
package main

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"os"
	"time"

	"github.com/hashicorp/terraform-plugin-sdk/v2/diag"
	"github.com/hashicorp/terraform-plugin-sdk/v2/helper/schema"
	"github.com/hashicorp/terraform-plugin-sdk/v2/plugin"
)

func main() {
	plugin.Serve(&plugin.ServeOpts{
		ProviderFunc: Provider,
	})
}

// Provider returns the ZVault Terraform provider schema.
func Provider() *schema.Provider {
	return &schema.Provider{
		Schema: map[string]*schema.Schema{
			"token": {
				Type:        schema.TypeString,
				Required:    true,
				Sensitive:   true,
				DefaultFunc: schema.EnvDefaultFunc("ZVAULT_TOKEN", nil),
				Description: "ZVault service token (zvt_...)",
			},
			"org_id": {
				Type:        schema.TypeString,
				Required:    true,
				DefaultFunc: schema.EnvDefaultFunc("ZVAULT_ORG_ID", nil),
				Description: "ZVault organization ID",
			},
			"url": {
				Type:        schema.TypeString,
				Optional:    true,
				DefaultFunc: schema.EnvDefaultFunc("ZVAULT_URL", "https://api.zvault.cloud"),
				Description: "ZVault Cloud API URL",
			},
		},
		ResourcesMap: map[string]*schema.Resource{
			"zvault_secret": resourceSecret(),
		},
		DataSourcesMap: map[string]*schema.Resource{
			"zvault_secret":  dataSourceSecret(),
			"zvault_secrets": dataSourceSecrets(),
		},
		ConfigureContextFunc: providerConfigure,
	}
}

// Client holds the ZVault API client configuration.
type Client struct {
	Token   string
	OrgID   string
	BaseURL string
	HTTP    *http.Client
}

func providerConfigure(_ context.Context, d *schema.ResourceData) (interface{}, diag.Diagnostics) {
	token := d.Get("token").(string)
	orgID := d.Get("org_id").(string)
	baseURL := d.Get("url").(string)

	return &Client{
		Token:   token,
		OrgID:   orgID,
		BaseURL: baseURL,
		HTTP:    &http.Client{Timeout: 30 * time.Second},
	}, nil
}

func (c *Client) apiGet(ctx context.Context, path string) ([]byte, error) {
	url := fmt.Sprintf("%s/v1/cloud%s", c.BaseURL, path)
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, url, nil)
	if err != nil {
		return nil, err
	}
	req.Header.Set("Authorization", "Bearer "+c.Token)
	req.Header.Set("User-Agent", "zvault-terraform/0.1.0")

	resp, err := c.HTTP.Do(req)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, err
	}

	if resp.StatusCode >= 400 {
		return nil, fmt.Errorf("ZVault API error %d: %s", resp.StatusCode, string(body))
	}
	return body, nil
}

// --- Data Source: zvault_secret ---

func dataSourceSecret() *schema.Resource {
	return &schema.Resource{
		ReadContext: dataSourceSecretRead,
		Schema: map[string]*schema.Schema{
			"project":     {Type: schema.TypeString, Required: true},
			"environment": {Type: schema.TypeString, Required: true},
			"key":         {Type: schema.TypeString, Required: true},
			"value":       {Type: schema.TypeString, Computed: true, Sensitive: true},
			"version":     {Type: schema.TypeInt, Computed: true},
			"updated_at":  {Type: schema.TypeString, Computed: true},
		},
	}
}

func dataSourceSecretRead(ctx context.Context, d *schema.ResourceData, meta interface{}) diag.Diagnostics {
	client := meta.(*Client)
	project := d.Get("project").(string)
	env := d.Get("environment").(string)
	key := d.Get("key").(string)

	path := fmt.Sprintf("/orgs/%s/projects/%s/envs/%s/secrets/%s", client.OrgID, project, env, key)
	body, err := client.apiGet(ctx, path)
	if err != nil {
		return diag.FromErr(err)
	}

	var resp struct {
		Secret struct {
			Key       string `json:"key"`
			Value     string `json:"value"`
			Version   int    `json:"version"`
			UpdatedAt string `json:"updated_at"`
		} `json:"secret"`
	}
	if err := json.Unmarshal(body, &resp); err != nil {
		return diag.FromErr(err)
	}

	d.SetId(fmt.Sprintf("%s/%s/%s", project, env, key))
	_ = d.Set("value", resp.Secret.Value)
	_ = d.Set("version", resp.Secret.Version)
	_ = d.Set("updated_at", resp.Secret.UpdatedAt)

	return nil
}

// --- Data Source: zvault_secrets ---

func dataSourceSecrets() *schema.Resource {
	return &schema.Resource{
		ReadContext: dataSourceSecretsRead,
		Schema: map[string]*schema.Schema{
			"project":     {Type: schema.TypeString, Required: true},
			"environment": {Type: schema.TypeString, Required: true},
			"secrets": {
				Type:      schema.TypeMap,
				Computed:  true,
				Sensitive: true,
				Elem:      &schema.Schema{Type: schema.TypeString},
			},
		},
	}
}

func dataSourceSecretsRead(ctx context.Context, d *schema.ResourceData, meta interface{}) diag.Diagnostics {
	client := meta.(*Client)
	project := d.Get("project").(string)
	env := d.Get("environment").(string)

	// Fetch keys
	keysPath := fmt.Sprintf("/orgs/%s/projects/%s/envs/%s/secrets", client.OrgID, project, env)
	keysBody, err := client.apiGet(ctx, keysPath)
	if err != nil {
		return diag.FromErr(err)
	}

	var keysResp struct {
		Keys []struct {
			Key string `json:"key"`
		} `json:"keys"`
	}
	if err := json.Unmarshal(keysBody, &keysResp); err != nil {
		return diag.FromErr(err)
	}

	// Fetch each secret
	secrets := make(map[string]string, len(keysResp.Keys))
	for _, k := range keysResp.Keys {
		secretPath := fmt.Sprintf("%s/%s", keysPath, k.Key)
		secretBody, err := client.apiGet(ctx, secretPath)
		if err != nil {
			continue
		}
		var secretResp struct {
			Secret struct {
				Value string `json:"value"`
			} `json:"secret"`
		}
		if err := json.Unmarshal(secretBody, &secretResp); err != nil {
			continue
		}
		secrets[k.Key] = secretResp.Secret.Value
	}

	d.SetId(fmt.Sprintf("%s/%s", project, env))
	_ = d.Set("secrets", secrets)

	return nil
}

// --- Resource: zvault_secret ---

func resourceSecret() *schema.Resource {
	return &schema.Resource{
		CreateContext: resourceSecretCreate,
		ReadContext:   resourceSecretRead,
		UpdateContext: resourceSecretUpdate,
		DeleteContext: resourceSecretDelete,
		Importer: &schema.ResourceImporter{
			StateContext: schema.ImportStatePassthroughContext,
		},
		Schema: map[string]*schema.Schema{
			"project":     {Type: schema.TypeString, Required: true, ForceNew: true},
			"environment": {Type: schema.TypeString, Required: true, ForceNew: true},
			"key":         {Type: schema.TypeString, Required: true, ForceNew: true},
			"value":       {Type: schema.TypeString, Required: true, Sensitive: true},
			"comment":     {Type: schema.TypeString, Optional: true, Default: ""},
			"version":     {Type: schema.TypeInt, Computed: true},
		},
	}
}

func resourceSecretCreate(ctx context.Context, d *schema.ResourceData, meta interface{}) diag.Diagnostics {
	// Create and update use the same PUT endpoint
	return resourceSecretUpdate(ctx, d, meta)
}

func resourceSecretRead(ctx context.Context, d *schema.ResourceData, meta interface{}) diag.Diagnostics {
	return dataSourceSecretRead(ctx, d, meta)
}

func resourceSecretUpdate(ctx context.Context, d *schema.ResourceData, meta interface{}) diag.Diagnostics {
	client := meta.(*Client)
	project := d.Get("project").(string)
	env := d.Get("environment").(string)
	key := d.Get("key").(string)
	value := d.Get("value").(string)
	comment := d.Get("comment").(string)

	path := fmt.Sprintf("/orgs/%s/projects/%s/envs/%s/secrets/%s", client.OrgID, project, env, key)
	url := fmt.Sprintf("%s/v1/cloud%s", client.BaseURL, path)

	payload := fmt.Sprintf(`{"value":%q,"comment":%q}`, value, comment)
	req, err := http.NewRequestWithContext(ctx, http.MethodPut, url, io.NopCloser(
		io.Reader(nil),
	))
	if err != nil {
		return diag.FromErr(err)
	}

	// Use strings.NewReader for the body
	req, err = http.NewRequestWithContext(ctx, http.MethodPut, url, nil)
	if err != nil {
		return diag.FromErr(err)
	}
	req.Body = io.NopCloser(stringReader(payload))
	req.Header.Set("Authorization", "Bearer "+client.Token)
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("User-Agent", "zvault-terraform/0.1.0")

	resp, err := client.HTTP.Do(req)
	if err != nil {
		return diag.FromErr(err)
	}
	defer resp.Body.Close()

	if resp.StatusCode >= 400 {
		body, _ := io.ReadAll(resp.Body)
		return diag.Errorf("ZVault API error %d: %s", resp.StatusCode, string(body))
	}

	d.SetId(fmt.Sprintf("%s/%s/%s", project, env, key))
	return resourceSecretRead(ctx, d, meta)
}

func resourceSecretDelete(ctx context.Context, d *schema.ResourceData, meta interface{}) diag.Diagnostics {
	client := meta.(*Client)
	project := d.Get("project").(string)
	env := d.Get("environment").(string)
	key := d.Get("key").(string)

	path := fmt.Sprintf("/orgs/%s/projects/%s/envs/%s/secrets/%s", client.OrgID, project, env, key)
	url := fmt.Sprintf("%s/v1/cloud%s", client.BaseURL, path)

	req, err := http.NewRequestWithContext(ctx, http.MethodDelete, url, nil)
	if err != nil {
		return diag.FromErr(err)
	}
	req.Header.Set("Authorization", "Bearer "+client.Token)
	req.Header.Set("User-Agent", "zvault-terraform/0.1.0")

	resp, err := client.HTTP.Do(req)
	if err != nil {
		return diag.FromErr(err)
	}
	defer resp.Body.Close()

	d.SetId("")
	return nil
}

// stringReader wraps a string as an io.Reader.
func stringReader(s string) io.Reader {
	return &stringReaderImpl{data: []byte(s)}
}

type stringReaderImpl struct {
	data []byte
	pos  int
}

func (r *stringReaderImpl) Read(p []byte) (int, error) {
	if r.pos >= len(r.data) {
		return 0, io.EOF
	}
	n := copy(p, r.data[r.pos:])
	r.pos += n
	return n, nil
}

// Ensure os is used (for env var fallback in provider schema).
var _ = os.Getenv
