package api

import (
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"sync"
	"time"

	"google.golang.org/protobuf/proto"
)

const (
	consentValidityDefault = 24 // hours
)

// PluginManager manages plugins and consent
type PluginManager struct {
	basePath string

	mu        sync.RWMutex
	plugins   map[string]*Plugin
	sessions  map[string]*Session
	consents  map[string]*ConsentRequest
	manifests map[string]*PluginManifest
}

type Plugin struct {
	ID          string
	Name        string
	Version     string
	IsApproved  bool
	Manifest    *PluginManifest
	ApprovedAt  time.Time
	ApprovedFields []string
}

type ConsentRequest struct {
	ID        string
	PluginID  string
	Fields    []string
	Status    string // pending, approved, denied
	CreatedAt time.Time
}

type Session struct {
	ID        string
	PluginID  string
	Fields    []string
	CreatedAt time.Time
	ExpiresAt time.Time
	Revoked   bool
}

type PluginManifest struct {
	ID            string   `json:"id"`
	Name          string   `json:"name"`
	Version       string   `json:"version"`
	Description   string   `json:"description"`
	Publisher     string   `json:"publisher"`
	Homepage      string   `json:"homepage"`
	Signature     string   `json:"signature"`
	RequiredFields []string `json:"required_fields"`
	OptionalFields []string `json:"optional_fields"`
	RequiresConsent bool   `json:"requires_consent"`
	ConsentValidityHours int `json:"consent_validity_hours"`
}

// NewPluginManager creates a new plugin manager
func NewPluginManager(basePath string) (*PluginManager, error) {
	pm := &PluginManager{
		basePath: basePath,
		plugins:  make(map[string]*Plugin),
		sessions: make(map[string]*Session),
		consents: make(map[string]*ConsentRequest),
		manifests: make(map[string]*PluginManifest),
	}

	// Load existing plugins
	if err := pm.loadPlugins(); err != nil {
		// Ignore if directory doesn't exist
	}

	return pm, nil
}

func (pm *PluginManager) manifestsPath() string {
	return filepath.Join(pm.basePath, "plugins", "manifests")
}

func (pm *PluginManager) approvedPath() string {
	return filepath.Join(pm.basePath, "plugins", "approved")
}

func (pm *PluginManager) sessionsPath() string {
	return filepath.Join(pm.basePath, "plugins", "sessions")
}

func (pm *PluginManager) loadPlugins() error {
	manifestsDir := pm.manifestsPath()
	entries, err := os.ReadDir(manifestsDir)
	if err != nil {
		if os.IsNotExist(err) {
			return nil
		}
		return err
	}

	for _, entry := range entries {
		if entry.IsDir() {
			manifestPath := filepath.Join(manifestsDir, entry.Name(), "manifest.json")
			if data, err := os.ReadFile(manifestPath); err == nil {
				var m PluginManifest
				if err := json.Unmarshal(data, &m); err == nil {
					pm.manifests[m.ID] = &m
					pm.plugins[m.ID] = &Plugin{
						ID:       m.ID,
						Name:     m.Name,
						Version:  m.Version,
						Manifest: &m,
					}
				}
			}
		}
	}

	// Load approved plugins
	approvedDir := pm.approvedPath()
	if entries, err := os.ReadDir(approvedDir); err == nil {
		for _, entry := range entries {
			if !entry.IsDir() {
				continue
			}
			approvedFile := filepath.Join(approvedDir, entry.Name(), "approved.json")
			if data, err := os.ReadFile(approvedFile); err == nil {
				var approval ApprovedInfo
				if err := json.Unmarshal(data, &approval); err == nil {
					if plugin, ok := pm.plugins[entry.Name()]; ok {
						plugin.IsApproved = true
						plugin.ApprovedAt = approval.ApprovedAt
						plugin.ApprovedFields = approval.Fields
					}
				}
			}
		}
	}

	return nil
}

type ApprovedInfo struct {
	ApprovedAt time.Time `json:"approved_at"`
	Fields     []string  `json:"fields"`
}

// RegisterPlugin registers a new plugin
func (pm *PluginManager) RegisterPlugin(manifestPath string) error {
	data, err := os.ReadFile(manifestPath)
	if err != nil {
		return fmt.Errorf("failed to read manifest: %w", err)
	}

	var m PluginManifest
	if err := json.Unmarshal(data, &m); err != nil {
		return fmt.Errorf("failed to parse manifest: %w", err)
	}

	// Verify signature (placeholder - would verify against publisher key)
	if m.Signature == "" {
		return errors.New("manifest must be signed")
	}

	// Copy manifest to plugins directory
	pluginDir := filepath.Join(pm.manifestsPath(), m.ID)
	if err := os.MkdirAll(pluginDir, 0755); err != nil {
		return err
	}

	if err := os.WriteFile(filepath.Join(pluginDir, "manifest.json"), data, 0644); err != nil {
		return err
	}

	pm.mu.Lock()
	defer pm.mu.Unlock()

	pm.manifests[m.ID] = &m
	pm.plugins[m.ID] = &Plugin{
		ID:       m.ID,
		Name:     m.Name,
		Version:  m.Version,
		Manifest: &m,
	}

	return nil
}

// GetManifest returns a plugin's manifest
func (pm *PluginManager) GetManifest(pluginID string) (*PluginManifest, error) {
	pm.mu.RLock()
	defer pm.mu.RUnlock()

	if m, ok := pm.manifests[pluginID]; ok {
		return m, nil
	}
	return nil, fmt.Errorf("plugin not found: %s", pluginID)
}

// ListPlugins returns all registered plugins
func (pm *PluginManager) ListPlugins() ([]*Plugin, error) {
	pm.mu.RLock()
	defer pm.mu.RUnlock()

	var plugins []*Plugin
	for _, p := range pm.plugins {
		plugins = append(plugins, p)
	}
	return plugins, nil
}

// ApprovePlugin approves a plugin for field access
func (pm *PluginManager) ApprovePlugin(pluginID string, fields []string) error {
	pm.mu.Lock()
	defer pm.mu.Unlock()

	plugin, ok := pm.plugins[pluginID]
	if !ok {
		return fmt.Errorf("plugin not registered: %s", pluginID)
	}

	approvedDir := filepath.Join(pm.approvedPath(), pluginID)
	if err := os.MkdirAll(approvedDir, 0755); err != nil {
		return err
	}

	approval := ApprovedInfo{
		ApprovedAt: time.Now(),
		Fields:     fields,
	}

	data, err := json.Marshal(approval)
	if err != nil {
		return err
	}

	if err := os.WriteFile(filepath.Join(approvedDir, "approved.json"), data, 0644); err != nil {
		return err
	}

	plugin.IsApproved = true
	plugin.ApprovedAt = approval.ApprovedAt
	plugin.ApprovedFields = fields

	return nil
}

// RequestConsent creates a new consent request
func (pm *PluginManager) RequestConsent(pluginID string, fields []string) (string, error) {
	pm.mu.Lock()
	defer pm.mu.Unlock()

	plugin, ok := pm.plugins[pluginID]
	if !ok {
		return "", fmt.Errorf("plugin not registered: %s", pluginID)
	}

	if !plugin.IsApproved {
		return "", fmt.Errorf("plugin not approved: %s", pluginID)
	}

	requestID := generateRequestID()
	pm.consents[requestID] = &ConsentRequest{
		ID:        requestID,
		PluginID:  pluginID,
		Fields:    fields,
		Status:    "pending",
		CreatedAt: time.Now(),
	}

	return requestID, nil
}

// GrantConsent grants consent for a plugin
func (pm *PluginManager) GrantConsent(requestID string, authorizedFields []string, validityHours int) (string, time.Time, error) {
	pm.mu.Lock()
	defer pm.mu.Unlock()

	consent, ok := pm.consents[requestID]
	if !ok {
		return "", time.Time{}, fmt.Errorf("consent request not found: %s", requestID)
	}

	if validityHours <= 0 {
		validityHours = consentValidityDefault
	}

	expiresAt := time.Now().Add(time.Duration(validityHours) * time.Hour)

	sessionID := generateSessionID()
	pm.sessions[sessionID] = &Session{
		ID:        sessionID,
		PluginID:  consent.PluginID,
		Fields:    authorizedFields,
		CreatedAt: time.Now(),
		ExpiresAt: expiresAt,
		Revoked:   false,
	}

	consent.Status = "approved"

	return sessionID, expiresAt, nil
}

// RevokeConsent revokes a session
func (pm *PluginManager) RevokeConsent(sessionID string) error {
	pm.mu.Lock()
	defer pm.mu.Unlock()

	session, ok := pm.sessions[sessionID]
	if !ok {
		return fmt.Errorf("session not found: %s", sessionID)
	}

	session.Revoked = true
	return nil
}

// ListSessions returns all sessions for a plugin
func (pm *PluginManager) ListSessions(pluginID string) ([]*Session, error) {
	pm.mu.RLock()
	defer pm.mu.RUnlock()

	var sessions []*Session
	for _, s := range pm.sessions {
		if s.PluginID == pluginID {
			sessions = append(sessions, s)
		}
	}
	return sessions, nil
}

// ValidateSession validates a session and returns it
func (pm *PluginManager) ValidateSession(sessionID string) (*Session, error) {
	pm.mu.RLock()
	defer pm.mu.RUnlock()

	session, ok := pm.sessions[sessionID]
	if !ok {
		return nil, fmt.Errorf("session not found: %s", sessionID)
	}

	if session.Revoked {
		return nil, fmt.Errorf("session revoked")
	}

	if time.Now().After(session.ExpiresAt) {
		return nil, fmt.Errorf("session expired")
	}

	return session, nil
}

// Helper functions

func generateRequestID() string {
	return fmt.Sprintf("req_%d", time.Now().UnixNano())
}

func generateSessionID() string {
	return fmt.Sprintf("sess_%d", time.Now().UnixNano())
}

// Protobuf conversion helpers

func manifestToProto(m *PluginManifest) *PluginManifest {
	return &PluginManifest{}
}

func protoToManifest(m proto.Message) *PluginManifest {
	return &PluginManifest{}
}
