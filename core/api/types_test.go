package api

import (
	"testing"
	"time"
)

func TestUnlockRequest(t *testing.T) {
	req := &UnlockRequest{
		MasterPassword:   "password123",
		ClientTimestamp: 1234567890,
	}

	if req.MasterPassword != "password123" {
		t.Errorf("MasterPassword = %q, want %q", req.MasterPassword, "password123")
	}
}

func TestUnlockResponse(t *testing.T) {
	resp := &UnlockResponse{
		Success:          true,
		SessionToken:     "token123",
		SessionExpiresAt: 1234567890,
		Error:            "",
	}

	if !resp.Success {
		t.Error("Success should be true")
	}
	if resp.SessionToken != "token123" {
		t.Errorf("SessionToken = %q, want %q", resp.SessionToken, "token123")
	}
}

func TestLockResponse(t *testing.T) {
	resp := &LockResponse{Success: true}
	if !resp.Success {
		t.Error("Success should be true")
	}
}

func TestChangePasswordResponse(t *testing.T) {
	resp := &ChangePasswordResponse{Success: true}
	if !resp.Success {
		t.Error("Success should be true")
	}
}

func TestGetProfileResponse(t *testing.T) {
	resp := &GetProfileResponse{
		Success: true,
		Profile: []byte(`{"profile_id": "test"}`),
	}

	if !resp.Success {
		t.Error("Success should be true")
	}
	if string(resp.Profile) != `{"profile_id": "test"}` {
		t.Errorf("Profile = %q, want %q", string(resp.Profile), `{"profile_id": "test"}`)
	}
}

func TestFieldError(t *testing.T) {
	fe := &FieldError{
		Field:   "identity.full_name",
		Message: "field is required",
	}

	if fe.Field != "identity.full_name" {
		t.Errorf("Field = %q, want %q", fe.Field, "identity.full_name")
	}
	if fe.Message != "field is required" {
		t.Errorf("Message = %q, want %q", fe.Message, "field is required")
	}
}

func TestValidateProfileResponse(t *testing.T) {
	resp := &ValidateProfileResponse{
		Valid:  false,
		Errors: []FieldError{{Field: "field1", Message: "error1"}},
	}

	if resp.Valid {
		t.Error("Valid should be false")
	}
	if len(resp.Errors) != 1 {
		t.Errorf("Errors len = %d, want 1", len(resp.Errors))
	}
}

func TestListProfilesResponse(t *testing.T) {
	resp := &ListProfilesResponse{
		ProfileIDs: []string{"profile1", "profile2"},
	}

	if len(resp.ProfileIDs) != 2 {
		t.Errorf("ProfileIDs len = %d, want 2", len(resp.ProfileIDs))
	}
}

func TestGetFieldsResponse(t *testing.T) {
	resp := &GetFieldsResponse{
		Fields: []*FieldValue{
			{Path: "field1", Value: []byte("value1")},
			{Path: "field2", Value: []byte("value2")},
		},
	}

	if len(resp.Fields) != 2 {
		t.Errorf("Fields len = %d, want 2", len(resp.Fields))
	}
}

func TestFieldValue(t *testing.T) {
	fv := &FieldValue{
		Path:       "identity.full_name",
		Value:      []byte("John Doe"),
		Confidence: 95,
	}

	if fv.Path != "identity.full_name" {
		t.Errorf("Path = %q, want %q", fv.Path, "identity.full_name")
	}
	if string(fv.Value) != "John Doe" {
		t.Errorf("Value = %q, want %q", string(fv.Value), "John Doe")
	}
	if fv.Confidence != 95 {
		t.Errorf("Confidence = %d, want 95", fv.Confidence)
	}
}

func TestPluginInfo(t *testing.T) {
	pi := &PluginInfo{
		ID:         "plugin-1",
		Name:       "Test Plugin",
		Version:    "1.0.0",
		IsApproved: true,
	}

	if pi.ID != "plugin-1" {
		t.Errorf("ID = %q, want %q", pi.ID, "plugin-1")
	}
	if !pi.IsApproved {
		t.Error("IsApproved should be true")
	}
}

func TestListPluginsResponse(t *testing.T) {
	resp := &ListPluginsResponse{
		Plugins: []*PluginInfo{
			{ID: "plugin1", Name: "Plugin 1"},
			{ID: "plugin2", Name: "Plugin 2"},
		},
	}

	if len(resp.Plugins) != 2 {
		t.Errorf("Plugins len = %d, want 2", len(resp.Plugins))
	}
}

func TestRequestConsentResponse(t *testing.T) {
	resp := &RequestConsentResponse{
		RequestID:      "req_123",
		Status:         "pending",
		RequiredFields: []string{"field1", "field2"},
	}

	if resp.RequestID != "req_123" {
		t.Errorf("RequestID = %q, want %q", resp.RequestID, "req_123")
	}
	if resp.Status != "pending" {
		t.Errorf("Status = %q, want %q", resp.Status, "pending")
	}
}

func TestGrantConsentResponse(t *testing.T) {
	resp := &GrantConsentResponse{
		Success:   true,
		SessionID: "sess_123",
		ExpiresAt: 1234567890,
	}

	if !resp.Success {
		t.Error("Success should be true")
	}
	if resp.SessionID != "sess_123" {
		t.Errorf("SessionID = %q, want %q", resp.SessionID, "sess_123")
	}
}

func TestConsentSession(t *testing.T) {
	now := time.Now()
	cs := &ConsentSession{
		ID:        "sess_123",
		PluginID:  "plugin_456",
		Fields:    []string{"field1"},
		CreatedAt: now,
		ExpiresAt: now.Add(24 * time.Hour),
		Revoked:   false,
	}

	if cs.ID != "sess_123" {
		t.Errorf("ID = %q, want %q", cs.ID, "sess_123")
	}
	if cs.Revoked {
		t.Error("Revoked should be false")
	}
}

func TestOCRJobResponse(t *testing.T) {
	resp := &OCRJobResponse{
		JobID:  "job_123",
		Status: "pending",
	}

	if resp.JobID != "job_123" {
		t.Errorf("JobID = %q, want %q", resp.JobID, "job_123")
	}
	if resp.Status != "pending" {
		t.Errorf("Status = %q, want %q", resp.Status, "pending")
	}
}

func TestGetOCRResultResponse(t *testing.T) {
	resp := &GetOCRResultResponse{
		Success:         true,
		Status:          "completed",
		ExtractedFields: []*FieldValue{{Path: "field1", Value: []byte("value1")}},
	}

	if !resp.Success {
		t.Error("Success should be true")
	}
	if len(resp.ExtractedFields) != 1 {
		t.Errorf("ExtractedFields len = %d, want 1", len(resp.ExtractedFields))
	}
}

func TestAccessProfileResponse(t *testing.T) {
	resp := &AccessProfileResponse{
		Success: true,
		Fields:  []*FieldValue{},
	}

	if !resp.Success {
		t.Error("Success should be true")
	}
}

func TestDeleteDocumentResponse(t *testing.T) {
	resp := &DeleteDocumentResponse{Success: true}
	if !resp.Success {
		t.Error("Success should be true")
	}
}

func TestStoreDocumentResponse(t *testing.T) {
	resp := &StoreDocumentResponse{
		DocumentID: "doc_123",
		Success:    true,
	}

	if resp.DocumentID != "doc_123" {
		t.Errorf("DocumentID = %q, want %q", resp.DocumentID, "doc_123")
	}
}
