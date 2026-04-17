package api

import (
	"crypto/rand"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"sync"
	"time"
)

// AccountManager manages multiple accounts in the solosoul directory
type AccountManager struct {
	basePath string
	accounts []Account
	mu       sync.RWMutex
}

// Account represents a user account
type Account struct {
	ID           string    `json:"id"`
	Name         string    `json:"name"`
	CreatedAt    time.Time `json:"created_at"`
	LastAccessed time.Time `json:"last_accessed"`
}

// AccountsIndex is stored in accounts.json
type AccountsIndex struct {
	Accounts       []Account `json:"accounts"`
	DefaultAccount string    `json:"default_account"`
}

const (
	accountsFileName = "accounts.json"
	accountPrefix    = "acc_"
)

// NewAccountManager creates a new account manager
func NewAccountManager(basePath string) (*AccountManager, error) {
	am := &AccountManager{
		basePath: basePath,
		accounts: []Account{},
	}

	// Ensure base directory exists
	if err := os.MkdirAll(basePath, 0700); err != nil {
		return nil, fmt.Errorf("failed to create base directory: %w", err)
	}

	// Load existing accounts
	if err := am.loadAccounts(); err != nil {
		return nil, fmt.Errorf("failed to load accounts: %w", err)
	}

	return am, nil
}

func (am *AccountManager) accountsPath() string {
	return filepath.Join(am.basePath, accountsFileName)
}

// loadAccounts loads accounts from disk
func (am *AccountManager) loadAccounts() error {
	am.mu.Lock()
	defer am.mu.Unlock()

	data, err := os.ReadFile(am.accountsPath())
	if err != nil {
		if os.IsNotExist(err) {
			// No accounts file yet - check for legacy vault
			if err := am.migrateLegacyVault(); err != nil {
				return fmt.Errorf("legacy migration failed: %w", err)
			}
			return nil
		}
		return err
	}

	var index AccountsIndex
	if err := json.Unmarshal(data, &index); err != nil {
		return fmt.Errorf("failed to parse accounts index: %w", err)
	}

	am.accounts = index.Accounts
	return nil
}

// saveAccounts saves accounts to disk
func (am *AccountManager) saveAccounts() error {
	index := AccountsIndex{
		Accounts: am.accounts,
	}

	// Find default account
	for _, acc := range am.accounts {
		if acc.ID == am.getDefaultAccountID() {
			index.DefaultAccount = acc.ID
			break
		}
	}

	data, err := json.MarshalIndent(index, "", "  ")
	if err != nil {
		return fmt.Errorf("failed to marshal accounts: %w", err)
	}

	return os.WriteFile(am.accountsPath(), data, 0600)
}

// migrateLegacyVault migrates the old single-vault format to the new multi-account format
func (am *AccountManager) migrateLegacyVault() error {
	configPath := filepath.Join(am.basePath, "config.json")

	// Check if legacy config exists
	if _, err := os.Stat(configPath); os.IsNotExist(err) {
		// No legacy vault, create default account
		am.accounts = []Account{}
		return nil
	}

	// Create default account
	defaultAccount := Account{
		ID:           "default",
		Name:         "Default",
		CreatedAt:    time.Now(),
		LastAccessed: time.Now(),
	}

	// Create default account directory
	defaultPath := filepath.Join(am.basePath, "default")
	if err := os.MkdirAll(defaultPath, 0700); err != nil {
		return fmt.Errorf("failed to create default account directory: %w", err)
	}

	// Move files to default account directory
	files := []string{"config.json", "index.db"}
	for _, file := range files {
		src := filepath.Join(am.basePath, file)
		dst := filepath.Join(defaultPath, file)
		if _, err := os.Stat(src); err == nil {
			if err := os.Rename(src, dst); err != nil {
				return fmt.Errorf("failed to migrate %s: %w", file, err)
			}
		}
	}

	// Move profiles directory if exists
	profilesSrc := filepath.Join(am.basePath, "profiles")
	profilesDst := filepath.Join(defaultPath, "profiles")
	if _, err := os.Stat(profilesSrc); err == nil {
		if err := os.Rename(profilesSrc, profilesDst); err != nil {
			return fmt.Errorf("failed to migrate profiles: %w", err)
		}
	}

	// Move blobs directory if exists
	blobsSrc := filepath.Join(am.basePath, "blobs")
	blobsDst := filepath.Join(defaultPath, "blobs")
	if _, err := os.Stat(blobsSrc); err == nil {
		if err := os.Rename(blobsSrc, blobsDst); err != nil {
			return fmt.Errorf("failed to migrate blobs: %w", err)
		}
	}

	am.accounts = []Account{defaultAccount}
	return am.saveAccounts()
}

// ListAccounts returns all accounts
func (am *AccountManager) ListAccounts() ([]Account, error) {
	am.mu.RLock()
	defer am.mu.RUnlock()

	result := make([]Account, len(am.accounts))
	copy(result, am.accounts)
	return result, nil
}

// GetAccount returns an account by ID
func (am *AccountManager) GetAccount(id string) (*Account, error) {
	am.mu.RLock()
	defer am.mu.RUnlock()

	for _, acc := range am.accounts {
		if acc.ID == id {
			return &acc, nil
		}
	}
	return nil, fmt.Errorf("account not found: %s", id)
}

// GetAccountByName returns an account by name (case-insensitive)
func (am *AccountManager) GetAccountByName(name string) (*Account, error) {
	am.mu.RLock()
	defer am.mu.RUnlock()

	for _, acc := range am.accounts {
		if strings.EqualFold(acc.Name, name) {
			return &acc, nil
		}
	}
	return nil, fmt.Errorf("account not found: %s", name)
}

// CreateAccount creates a new account
func (am *AccountManager) CreateAccount(name, password string) (*Account, error) {
	am.mu.Lock()
	defer am.mu.Unlock()

	// Generate unique account ID
	id, err := generateAccountID()
	if err != nil {
		return nil, fmt.Errorf("failed to generate account ID: %w", err)
	}

	// Create account directory
	accountPath := filepath.Join(am.basePath, id)
	if err := os.MkdirAll(accountPath, 0700); err != nil {
		return nil, fmt.Errorf("failed to create account directory: %w", err)
	}

	account := Account{
		ID:           id,
		Name:         name,
		CreatedAt:    time.Now(),
		LastAccessed: time.Now(),
	}

	am.accounts = append(am.accounts, account)

	if err := am.saveAccounts(); err != nil {
		// Clean up
		os.RemoveAll(accountPath)
		return nil, fmt.Errorf("failed to save accounts: %w", err)
	}

	return &account, nil
}

// DeleteAccount deletes an account
func (am *AccountManager) DeleteAccount(id string) error {
	am.mu.Lock()
	defer am.mu.Unlock()

	// Find and remove account
	var idx = -1
	for i, acc := range am.accounts {
		if acc.ID == id {
			idx = i
			break
		}
	}

	if idx < 0 {
		return fmt.Errorf("account not found: %s", id)
	}

	// Remove from slice
	am.accounts = append(am.accounts[:idx], am.accounts[idx+1:]...)

	// Delete account directory
	accountPath := filepath.Join(am.basePath, id)
	if err := os.RemoveAll(accountPath); err != nil {
		// Rollback
		am.accounts = append(am.accounts[:idx], append([]Account{{ID: id}}, am.accounts[idx:]...)...)
		return fmt.Errorf("failed to delete account directory: %w", err)
	}

	return am.saveAccounts()
}

// SetDefault sets the default account
func (am *AccountManager) SetDefault(id string) error {
	am.mu.Lock()
	defer am.mu.Unlock()

	// Verify account exists
	found := false
	for _, acc := range am.accounts {
		if acc.ID == id {
			found = true
			break
		}
	}
	if !found {
		return fmt.Errorf("account not found: %s", id)
	}

	return am.saveAccounts()
}

// UpdateLastAccessed updates the last accessed time for an account
func (am *AccountManager) UpdateLastAccessed(id string) error {
	am.mu.Lock()
	defer am.mu.Unlock()

	for i := range am.accounts {
		if am.accounts[i].ID == id {
			am.accounts[i].LastAccessed = time.Now()
			return am.saveAccounts()
		}
	}

	return fmt.Errorf("account not found: %s", id)
}

// GetAccountPath returns the vault path for an account
func (am *AccountManager) GetAccountPath(id string) (string, error) {
	am.mu.RLock()
	defer am.mu.RUnlock()

	for _, acc := range am.accounts {
		if acc.ID == id {
			return filepath.Join(am.basePath, acc.ID), nil
		}
	}

	return "", fmt.Errorf("account not found: %s", id)
}

// GetDefaultAccountID returns the default account ID
func (am *AccountManager) getDefaultAccountID() string {
	for _, acc := range am.accounts {
		if acc.ID == "default" {
			return acc.ID
		}
	}
	if len(am.accounts) > 0 {
		return am.accounts[0].ID
	}
	return ""
}

// GetDefaultAccount returns the default account
func (am *AccountManager) GetDefaultAccount() *Account {
	am.mu.RLock()
	defer am.mu.RUnlock()

	defaultID := am.getDefaultAccountID()
	for _, acc := range am.accounts {
		if acc.ID == defaultID {
			return &acc
		}
	}
	return nil
}

// generateAccountID generates a unique account ID
func generateAccountID() (string, error) {
	b := make([]byte, 4)
	if _, err := rand.Read(b); err != nil {
		return "", err
	}
	return accountPrefix + hex.EncodeToString(b), nil
}
