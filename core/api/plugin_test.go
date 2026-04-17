package api

import (
	"testing"
	"time"
)

func TestGenerateRequestID(t *testing.T) {
	id1 := generateRequestID()
	id2 := generateRequestID()

	if id1 == "" {
		t.Error("generateRequestID() returned empty string")
	}

	if id1 == id2 {
		t.Error("generateRequestID() should produce unique IDs")
	}

	// Should start with req_
	if len(id1) < 4 || id1[:4] != "req_" {
		t.Errorf("generateRequestID() = %q, should start with 'req_'", id1)
	}
}

func TestGenerateSessionID(t *testing.T) {
	id1 := generateSessionID()
	id2 := generateSessionID()

	if id1 == "" {
		t.Error("generateSessionID() returned empty string")
	}

	if id1 == id2 {
		t.Error("generateSessionID() should produce unique IDs")
	}

	// Should start with sess_
	if len(id1) < 5 || id1[:5] != "sess_" {
		t.Errorf("generateSessionID() = %q, should start with 'sess_'", id1)
	}
}

func TestPluginManifest_Fields(t *testing.T) {
	m := &PluginManifest{
		ID:              "test-plugin",
		Name:            "Test Plugin",
		Version:         "1.0.0",
		Description:     "A test plugin",
		Publisher:       "Test Publisher",
		Homepage:        "https://example.com",
		Signature:       "abc123",
		RequiredFields:  []string{"identity.full_name"},
		OptionalFields:  []string{"identity.email"},
		RequiresConsent: true,
		ConsentValidityHours: 24,
	}

	if m.ID != "test-plugin" {
		t.Errorf("ID = %q, want %q", m.ID, "test-plugin")
	}
	if m.RequiredFields[0] != "identity.full_name" {
		t.Errorf("RequiredFields[0] = %q, want %q", m.RequiredFields[0], "identity.full_name")
	}
}

func TestSession_Fields(t *testing.T) {
	now := time.Now()
	session := &Session{
		ID:        "sess_123",
		PluginID:  "plugin_456",
		Fields:    []string{"field1", "field2"},
		CreatedAt: now,
		ExpiresAt: now.Add(24 * time.Hour),
		Revoked:   false,
	}

	if session.ID != "sess_123" {
		t.Errorf("ID = %q, want %q", session.ID, "sess_123")
	}
	if session.PluginID != "plugin_456" {
		t.Errorf("PluginID = %q, want %q", session.PluginID, "plugin_456")
	}
	if len(session.Fields) != 2 {
		t.Errorf("Fields len = %d, want 2", len(session.Fields))
	}
	if session.Revoked {
		t.Error("Revoked should be false")
	}
}

func TestConsentRequest_Fields(t *testing.T) {
	now := time.Now()
	consent := &ConsentRequest{
		ID:        "req_123",
		PluginID:  "plugin_456",
		Fields:    []string{"field1"},
		Status:    "pending",
		CreatedAt: now,
	}

	if consent.ID != "req_123" {
		t.Errorf("ID = %q, want %q", consent.ID, "req_123")
	}
	if consent.Status != "pending" {
		t.Errorf("Status = %q, want %q", consent.Status, "pending")
	}
}

func TestPlugin_Fields(t *testing.T) {
	plugin := &Plugin{
		ID:          "test-plugin",
		Name:        "Test Plugin",
		Version:     "1.0.0",
		IsApproved:  false,
		ApprovedAt:  time.Time{},
		ApprovedFields: []string{},
	}

	if plugin.ID != "test-plugin" {
		t.Errorf("ID = %q, want %q", plugin.ID, "test-plugin")
	}
	if plugin.IsApproved {
		t.Error("IsApproved should be false initially")
	}
}

func TestIsFieldAllowed(t *testing.T) {
	tests := []struct {
		name          string
		fieldPath     string
		allowedFields []string
		want          bool
	}{
		{
			name:          "exact match",
			fieldPath:     "identity.full_name",
			allowedFields: []string{"identity.full_name", "identity.email"},
			want:          true,
		},
		{
			name:          "wildcard match",
			fieldPath:     "identity.any_field",
			allowedFields: []string{"*"},
			want:          true,
		},
		{
			name:          "no match",
			fieldPath:     "travel.visa",
			allowedFields: []string{"identity.full_name"},
			want:          false,
		},
		{
			name:          "empty allowed fields",
			fieldPath:     "identity.full_name",
			allowedFields: []string{},
			want:          false,
		},
		{
			name:          "partial match",
			fieldPath:     "identity.full_name",
			allowedFields: []string{"identity.full"},
			want:          false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := isFieldAllowed(tt.fieldPath, tt.allowedFields)
			if got != tt.want {
				t.Errorf("isFieldAllowed(%q, %v) = %v, want %v",
					tt.fieldPath, tt.allowedFields, got, tt.want)
			}
		})
	}
}

func TestApprovedInfo(t *testing.T) {
	now := time.Now()
	info := ApprovedInfo{
		ApprovedAt: now,
		Fields:    []string{"field1", "field2"},
	}

	if info.ApprovedAt != now {
		t.Errorf("ApprovedAt = %v, want %v", info.ApprovedAt, now)
	}
	if len(info.Fields) != 2 {
		t.Errorf("Fields len = %d, want 2", len(info.Fields))
	}
}

func TestPluginManager_NewPluginManager(t *testing.T) {
	pm, err := NewPluginManager("/tmp/test-plugins")
	if err != nil {
		t.Fatalf("NewPluginManager failed: %v", err)
	}

	if pm == nil {
		t.Fatal("NewPluginManager returned nil")
	}

	if pm.plugins == nil {
		t.Error("plugins map should be initialized")
	}
	if pm.sessions == nil {
		t.Error("sessions map should be initialized")
	}
	if pm.consents == nil {
		t.Error("consents map should be initialized")
	}
	if pm.manifests == nil {
		t.Error("manifests map should be initialized")
	}
}

func TestPluginManager_GetManifest_NotFound(t *testing.T) {
	pm, err := NewPluginManager("/tmp/test-plugins")
	if err != nil {
		t.Fatalf("NewPluginManager failed: %v", err)
	}

	_, err = pm.GetManifest("nonexistent-plugin")
	if err == nil {
		t.Error("GetManifest() for nonexistent plugin should return error")
	}
}

func TestPluginManager_ListPlugins_Empty(t *testing.T) {
	pm, err := NewPluginManager("/tmp/test-plugins")
	if err != nil {
		t.Fatalf("NewPluginManager failed: %v", err)
	}

	plugins, err := pm.ListPlugins()
	if err != nil {
		t.Fatalf("ListPlugins() failed: %v", err)
	}

	if len(plugins) != 0 {
		t.Errorf("ListPlugins() returned %d plugins, want 0", len(plugins))
	}
}

func TestPluginManager_RequestConsent_PluginNotRegistered(t *testing.T) {
	pm, err := NewPluginManager("/tmp/test-plugins")
	if err != nil {
		t.Fatalf("NewPluginManager failed: %v", err)
	}

	_, err = pm.RequestConsent("nonexistent-plugin", []string{"field1"})
	if err == nil {
		t.Error("RequestConsent() for unregistered plugin should return error")
	}
}

func TestPluginManager_ApprovePlugin_NotRegistered(t *testing.T) {
	pm, err := NewPluginManager("/tmp/test-plugins")
	if err != nil {
		t.Fatalf("NewPluginManager failed: %v", err)
	}

	err = pm.ApprovePlugin("nonexistent-plugin", []string{"field1"})
	if err == nil {
		t.Error("ApprovePlugin() for unregistered plugin should return error")
	}
}

func TestPluginManager_RevokeConsent_NotFound(t *testing.T) {
	pm, err := NewPluginManager("/tmp/test-plugins")
	if err != nil {
		t.Fatalf("NewPluginManager failed: %v", err)
	}

	err = pm.RevokeConsent("nonexistent-session")
	if err == nil {
		t.Error("RevokeConsent() for nonexistent session should return error")
	}
}

func TestPluginManager_ValidateSession_NotFound(t *testing.T) {
	pm, err := NewPluginManager("/tmp/test-plugins")
	if err != nil {
		t.Fatalf("NewPluginManager failed: %v", err)
	}

	_, err = pm.ValidateSession("nonexistent-session")
	if err == nil {
		t.Error("ValidateSession() for nonexistent session should return error")
	}
}

func TestConsentValidityDefault(t *testing.T) {
	if consentValidityDefault != 24 {
		t.Errorf("consentValidityDefault = %d, want 24", consentValidityDefault)
	}
}
