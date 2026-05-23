package api

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
	"time"
)

func newTestPluginManager(t *testing.T) *PluginManager {
	t.Helper()
	dir := t.TempDir()
	pm, err := NewPluginManager(dir)
	if err != nil {
		t.Fatalf("NewPluginManager() error = %v", err)
	}
	return pm
}

func TestPluginManager_RegisterPlugin(t *testing.T) {
	t.Run("registers valid manifest", func(t *testing.T) {
		pm := newTestPluginManager(t)
		manifest := &PluginManifest{
			ID:        "com.test.plugin",
			Name:      "Test Plugin",
			Version:   "1.0.0",
			Signature: "sig123",
		}
		data, _ := json.Marshal(manifest)
		tmpFile := filepath.Join(t.TempDir(), "manifest.json")
		os.WriteFile(tmpFile, data, 0644)

		err := pm.RegisterPlugin(tmpFile)
		if err != nil {
			t.Fatalf("RegisterPlugin() error = %v", err)
		}

		// Verify plugin was registered
		p, err := pm.GetManifest("com.test.plugin")
		if err != nil {
			t.Fatalf("GetManifest() error = %v", err)
		}
		if p.Name != "Test Plugin" {
			t.Errorf("expected name 'Test Plugin', got %q", p.Name)
		}
	})

	t.Run("rejects unsigned manifest", func(t *testing.T) {
		pm := newTestPluginManager(t)
		manifest := &PluginManifest{
			ID:      "com.test.plugin2",
			Name:    "Test Plugin 2",
			Version: "1.0.0",
			// No signature
		}
		data, _ := json.Marshal(manifest)
		tmpFile := filepath.Join(t.TempDir(), "manifest.json")
		os.WriteFile(tmpFile, data, 0644)

		err := pm.RegisterPlugin(tmpFile)
		if err == nil {
			t.Error("RegisterPlugin() expected error for unsigned manifest")
		}
	})

	t.Run("rejects missing file", func(t *testing.T) {
		pm := newTestPluginManager(t)
		err := pm.RegisterPlugin("/nonexistent/manifest.json")
		if err == nil {
			t.Error("RegisterPlugin() expected error for missing file")
		}
	})
}

func TestPluginManager_ApprovePlugin(t *testing.T) {
	t.Run("approves registered plugin", func(t *testing.T) {
		pm := newTestPluginManager(t)
		manifest := &PluginManifest{
			ID:        "com.test.plugin",
			Name:      "Test Plugin",
			Version:   "1.0.0",
			Signature: "sig123",
		}
		data, _ := json.Marshal(manifest)
		tmpFile := filepath.Join(t.TempDir(), "manifest.json")
		os.WriteFile(tmpFile, data, 0644)
		pm.RegisterPlugin(tmpFile)

		fields := []string{"identity.full_name", "identity.date_of_birth"}
		err := pm.ApprovePlugin("com.test.plugin", fields)
		if err != nil {
			t.Fatalf("ApprovePlugin() error = %v", err)
		}

		// Verify plugin is approved
		plugins, _ := pm.ListPlugins()
		for _, p := range plugins {
			if p.ID == "com.test.plugin" && !p.IsApproved {
				t.Error("plugin should be approved")
			}
		}
	})

	t.Run("rejects unregistered plugin", func(t *testing.T) {
		pm := newTestPluginManager(t)
		err := pm.ApprovePlugin("nonexistent", []string{"field1"})
		if err == nil {
			t.Error("ApprovePlugin() expected error for unregistered plugin")
		}
	})
}

func TestPluginManager_ConsentFlow(t *testing.T) {
	pm := newTestPluginManager(t)

	// Register and approve a plugin
	manifest := &PluginManifest{
		ID:        "com.test.plugin",
		Name:      "Test Plugin",
		Version:   "1.0.0",
		Signature: "sig123",
	}
	data, _ := json.Marshal(manifest)
	tmpFile := filepath.Join(t.TempDir(), "manifest.json")
	os.WriteFile(tmpFile, data, 0644)
	pm.RegisterPlugin(tmpFile)
	pm.ApprovePlugin("com.test.plugin", []string{"identity.full_name"})

	t.Run("RequestConsent creates pending request", func(t *testing.T) {
		reqID, err := pm.RequestConsent("com.test.plugin", []string{"identity.full_name"})
		if err != nil {
			t.Fatalf("RequestConsent() error = %v", err)
		}
		if reqID == "" {
			t.Error("RequestConsent() returned empty request ID")
		}
		if len(reqID) < 4 || reqID[:4] != "req_" {
			t.Errorf("request ID %q should start with 'req_'", reqID)
		}
	})

	t.Run("RequestConsent rejects unapproved plugin", func(t *testing.T) {
		_, err := pm.RequestConsent("com.unapproved.plugin", []string{"field1"})
		if err == nil {
			t.Error("RequestConsent() expected error for unapproved plugin")
		}
	})

	t.Run("GrantConsent creates session", func(t *testing.T) {
		reqID, _ := pm.RequestConsent("com.test.plugin", []string{"identity.full_name"})

		sessionID, expiresAt, err := pm.GrantConsent(reqID, []string{"identity.full_name"}, 1)
		if err != nil {
			t.Fatalf("GrantConsent() error = %v", err)
		}
		if sessionID == "" {
			t.Error("GrantConsent() returned empty session ID")
		}
		if expiresAt.Before(time.Now()) {
			t.Error("GrantConsent() returned expired session")
		}
		if len(sessionID) < 5 || sessionID[:5] != "sess_" {
			t.Errorf("session ID %q should start with 'sess_'", sessionID)
		}
	})

	t.Run("GrantConsent uses default validity", func(t *testing.T) {
		reqID, _ := pm.RequestConsent("com.test.plugin", []string{"identity.full_name"})

		_, expiresAt, err := pm.GrantConsent(reqID, []string{"identity.full_name"}, 0)
		if err != nil {
			t.Fatalf("GrantConsent() error = %v", err)
		}
		// Should be ~24 hours from now
		expectedMax := time.Now().Add(25 * time.Hour)
		if expiresAt.After(expectedMax) {
			t.Error("GrantConsent() with 0 validity should use default 24h")
		}
	})

	t.Run("GrantConsent rejects invalid request ID", func(t *testing.T) {
		_, _, err := pm.GrantConsent("invalid-req", []string{"field1"}, 1)
		if err == nil {
			t.Error("GrantConsent() expected error for invalid request")
		}
	})
}

func TestPluginManager_SessionManagement(t *testing.T) {
	pm := newTestPluginManager(t)

	// Setup: register, approve, request, grant
	manifest := &PluginManifest{
		ID:        "com.test.plugin",
		Name:      "Test Plugin",
		Version:   "1.0.0",
		Signature: "sig123",
	}
	data, _ := json.Marshal(manifest)
	tmpFile := filepath.Join(t.TempDir(), "manifest.json")
	os.WriteFile(tmpFile, data, 0644)
	pm.RegisterPlugin(tmpFile)
	pm.ApprovePlugin("com.test.plugin", []string{"identity.full_name"})

	reqID, _ := pm.RequestConsent("com.test.plugin", []string{"identity.full_name"})
	sessionID, _, _ := pm.GrantConsent(reqID, []string{"identity.full_name"}, 1)

	t.Run("ValidateSession returns valid session", func(t *testing.T) {
		session, err := pm.ValidateSession(sessionID)
		if err != nil {
			t.Fatalf("ValidateSession() error = %v", err)
		}
		if session == nil {
			t.Fatal("ValidateSession() returned nil")
		}
		if session.ID != sessionID {
			t.Errorf("expected session ID %s, got %s", sessionID, session.ID)
		}
		if session.PluginID != "com.test.plugin" {
			t.Errorf("expected plugin ID com.test.plugin, got %s", session.PluginID)
		}
	})

	t.Run("RevokeConsent invalidates session", func(t *testing.T) {
		err := pm.RevokeConsent(sessionID)
		if err != nil {
			t.Fatalf("RevokeConsent() error = %v", err)
		}

		_, err = pm.ValidateSession(sessionID)
		if err == nil {
			t.Error("ValidateSession() expected error after revocation")
		}
	})

	t.Run("ValidateSession rejects expired session", func(t *testing.T) {
		// Create a new session, then manually expire it
		reqID2, _ := pm.RequestConsent("com.test.plugin", []string{"identity.full_name"})
		sessionID2, _, _ := pm.GrantConsent(reqID2, []string{"identity.full_name"}, 1)

		// Manually expire the session
		pm.mu.Lock()
		pm.sessions[sessionID2].ExpiresAt = time.Now().Add(-1 * time.Hour)
		pm.mu.Unlock()

		_, err := pm.ValidateSession(sessionID2)
		if err == nil {
			t.Error("ValidateSession() expected error for expired session")
		}
	})

	t.Run("ListSessions returns plugin sessions", func(t *testing.T) {
		// Create fresh session
		reqID3, _ := pm.RequestConsent("com.test.plugin", []string{"identity.full_name"})
		pm.GrantConsent(reqID3, []string{"identity.full_name"}, 1)

		sessions, err := pm.ListSessions("com.test.plugin")
		if err != nil {
			t.Fatalf("ListSessions() error = %v", err)
		}
		if len(sessions) == 0 {
			t.Error("ListSessions() should return at least one session")
		}
	})

	t.Run("ListSessions returns empty for unknown plugin", func(t *testing.T) {
		sessions, err := pm.ListSessions("com.unknown.plugin")
		if err != nil {
			t.Fatalf("ListSessions() error = %v", err)
		}
		if len(sessions) != 0 {
			t.Errorf("expected 0 sessions, got %d", len(sessions))
		}
	})
}

func TestPluginManager_loadPlugins(t *testing.T) {
	t.Run("loads plugins from disk", func(t *testing.T) {
		dir := t.TempDir()
		_, _ = NewPluginManager(dir)

		// Create manifest directory and file
		manifestDir := filepath.Join(dir, "plugins", "manifests", "com.test.plugin")
		os.MkdirAll(manifestDir, 0755)
		manifest := &PluginManifest{
			ID:        "com.test.plugin",
			Name:      "Loaded Plugin",
			Version:   "1.0.0",
			Signature: "sig",
		}
		data, _ := json.Marshal(manifest)
		os.WriteFile(filepath.Join(manifestDir, "manifest.json"), data, 0644)

		// Create approved directory and file
		approvedDir := filepath.Join(dir, "plugins", "approved", "com.test.plugin")
		os.MkdirAll(approvedDir, 0755)
		approval := ApprovedInfo{
			ApprovedAt: time.Now(),
			Fields:     []string{"field1"},
		}
		approvalData, _ := json.Marshal(approval)
		os.WriteFile(filepath.Join(approvedDir, "approved.json"), approvalData, 0644)

		// Create new manager to trigger loadPlugins
		pm2, err := NewPluginManager(dir)
		if err != nil {
			t.Fatalf("NewPluginManager() error = %v", err)
		}

		plugins, _ := pm2.ListPlugins()
		if len(plugins) != 1 {
			t.Fatalf("expected 1 loaded plugin, got %d", len(plugins))
		}
		if plugins[0].Name != "Loaded Plugin" {
			t.Errorf("expected name 'Loaded Plugin', got %q", plugins[0].Name)
		}
		if !plugins[0].IsApproved {
			t.Error("loaded plugin should be approved")
		}
	})

	t.Run("handles missing plugin directories gracefully", func(t *testing.T) {
		dir := t.TempDir()
		pm, err := NewPluginManager(dir)
		if err != nil {
			t.Fatalf("NewPluginManager() error = %v", err)
		}
		plugins, _ := pm.ListPlugins()
		if len(plugins) != 0 {
			t.Errorf("expected 0 plugins, got %d", len(plugins))
		}
	})
}
