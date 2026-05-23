package api

import (
	"os"
	"path/filepath"
	"testing"
)

func TestNewAccountManager(t *testing.T) {
	t.Run("creates manager with empty directory", func(t *testing.T) {
		dir := t.TempDir()
		am, err := NewAccountManager(dir)
		if err != nil {
			t.Fatalf("NewAccountManager() error = %v", err)
		}
		if am == nil {
			t.Fatal("NewAccountManager() returned nil")
		}

		accounts, err := am.ListAccounts()
		if err != nil {
			t.Fatalf("ListAccounts() error = %v", err)
		}
		if len(accounts) != 0 {
			t.Errorf("expected 0 accounts, got %d", len(accounts))
		}
	})

	t.Run("loads existing accounts", func(t *testing.T) {
		dir := t.TempDir()

		// Pre-create accounts.json
		accountsData := `{"accounts":[{"id":"acc_1234","name":"Test","created_at":"2024-01-01T00:00:00Z","last_accessed":"2024-01-01T00:00:00Z"}],"default_account":"acc_1234"}`
		if err := os.WriteFile(filepath.Join(dir, "accounts.json"), []byte(accountsData), 0600); err != nil {
			t.Fatalf("failed to write accounts.json: %v", err)
		}

		am, err := NewAccountManager(dir)
		if err != nil {
			t.Fatalf("NewAccountManager() error = %v", err)
		}

		accounts, err := am.ListAccounts()
		if err != nil {
			t.Fatalf("ListAccounts() error = %v", err)
		}
		if len(accounts) != 1 {
			t.Errorf("expected 1 account, got %d", len(accounts))
		}
		if accounts[0].ID != "acc_1234" {
			t.Errorf("expected account ID acc_1234, got %s", accounts[0].ID)
		}
	})

	t.Run("migrates legacy vault", func(t *testing.T) {
		dir := t.TempDir()

		// Create legacy config.json
		if err := os.WriteFile(filepath.Join(dir, "config.json"), []byte(`{"version":1}`), 0600); err != nil {
			t.Fatalf("failed to write config.json: %v", err)
		}

		am, err := NewAccountManager(dir)
		if err != nil {
			t.Fatalf("NewAccountManager() error = %v", err)
		}

		accounts, err := am.ListAccounts()
		if err != nil {
			t.Fatalf("ListAccounts() error = %v", err)
		}
		if len(accounts) != 1 {
			t.Errorf("expected 1 migrated account, got %d", len(accounts))
		}
		if accounts[0].ID != "default" {
			t.Errorf("expected default account ID, got %s", accounts[0].ID)
		}
	})
}

func TestAccountManager_CreateAccount(t *testing.T) {
	am := newTestAccountManager(t)

	t.Run("creates account successfully", func(t *testing.T) {
		acc, err := am.CreateAccount("TestAccount", "password123")
		if err != nil {
			t.Fatalf("CreateAccount() error = %v", err)
		}
		if acc == nil {
			t.Fatal("CreateAccount() returned nil")
		}
		if acc.Name != "TestAccount" {
			t.Errorf("expected name TestAccount, got %s", acc.Name)
		}
		if acc.ID == "" {
			t.Error("expected non-empty account ID")
		}

		// Verify directory was created
		accPath, err := am.GetAccountPath(acc.ID)
		if err != nil {
			t.Fatalf("GetAccountPath() error = %v", err)
		}
		if _, err := os.Stat(accPath); os.IsNotExist(err) {
			t.Errorf("account directory not created: %s", accPath)
		}
	})

	t.Run("creates multiple accounts", func(t *testing.T) {
		am2 := newTestAccountManager(t)
		acc1, err := am2.CreateAccount("Account1", "password123")
		if err != nil {
			t.Fatalf("CreateAccount() error = %v", err)
		}
		acc2, err := am2.CreateAccount("Account2", "password123")
		if err != nil {
			t.Fatalf("CreateAccount() error = %v", err)
		}

		accounts, _ := am2.ListAccounts()
		if len(accounts) != 2 {
			t.Errorf("expected 2 accounts, got %d", len(accounts))
		}

		// Verify IDs are unique
		if acc1.ID == acc2.ID {
			t.Error("account IDs should be unique")
		}
	})
}

func TestAccountManager_GetAccount(t *testing.T) {
	am := newTestAccountManager(t)
	acc, _ := am.CreateAccount("TestAccount", "password123")

	t.Run("finds existing account", func(t *testing.T) {
		found, err := am.GetAccount(acc.ID)
		if err != nil {
			t.Fatalf("GetAccount() error = %v", err)
		}
		if found == nil {
			t.Fatal("GetAccount() returned nil")
		}
		if found.ID != acc.ID {
			t.Errorf("expected ID %s, got %s", acc.ID, found.ID)
		}
	})

	t.Run("returns error for non-existent account", func(t *testing.T) {
		_, err := am.GetAccount("nonexistent")
		if err == nil {
			t.Error("GetAccount() expected error for non-existent account")
		}
	})
}

func TestAccountManager_GetAccountByName(t *testing.T) {
	am := newTestAccountManager(t)
	_, _ = am.CreateAccount("MyAccount", "password123")

	t.Run("finds by exact name", func(t *testing.T) {
		found, err := am.GetAccountByName("MyAccount")
		if err != nil {
			t.Fatalf("GetAccountByName() error = %v", err)
		}
		if found == nil || found.Name != "MyAccount" {
			t.Error("GetAccountByName() did not find account")
		}
	})

	t.Run("finds by case-insensitive name", func(t *testing.T) {
		found, err := am.GetAccountByName("myaccount")
		if err != nil {
			t.Fatalf("GetAccountByName() error = %v", err)
		}
		if found == nil || found.Name != "MyAccount" {
			t.Error("GetAccountByName() did not find account case-insensitively")
		}
	})

	t.Run("returns error for non-existent name", func(t *testing.T) {
		_, err := am.GetAccountByName("NonExistent")
		if err == nil {
			t.Error("GetAccountByName() expected error")
		}
	})
}

func TestAccountManager_ListAccounts(t *testing.T) {
	am := newTestAccountManager(t)

	t.Run("returns empty list initially", func(t *testing.T) {
		accounts, err := am.ListAccounts()
		if err != nil {
			t.Fatalf("ListAccounts() error = %v", err)
		}
		if len(accounts) != 0 {
			t.Errorf("expected 0 accounts, got %d", len(accounts))
		}
	})

	t.Run("returns copy of accounts", func(t *testing.T) {
		_, _ = am.CreateAccount("Account1", "password123")
		accounts, _ := am.ListAccounts()

		// Modify returned slice - should not affect internal state
		accounts = append(accounts, Account{ID: "fake", Name: "Fake"})

		accounts2, _ := am.ListAccounts()
		if len(accounts2) != 1 {
			t.Error("ListAccounts() returned mutable slice")
		}
	})
}

func TestAccountManager_DeleteAccount(t *testing.T) {
	am := newTestAccountManager(t)
	acc, _ := am.CreateAccount("ToDelete", "password123")

	t.Run("deletes existing account", func(t *testing.T) {
		err := am.DeleteAccount(acc.ID)
		if err != nil {
			t.Fatalf("DeleteAccount() error = %v", err)
		}

		accounts, _ := am.ListAccounts()
		if len(accounts) != 0 {
			t.Errorf("expected 0 accounts after delete, got %d", len(accounts))
		}

		// Verify directory was removed
		accPath, _ := am.GetAccountPath(acc.ID)
		if _, err := os.Stat(accPath); !os.IsNotExist(err) {
			t.Error("account directory should be removed after delete")
		}
	})

	t.Run("returns error for non-existent account", func(t *testing.T) {
		err := am.DeleteAccount("nonexistent")
		if err == nil {
			t.Error("DeleteAccount() expected error for non-existent account")
		}
	})
}

func TestAccountManager_SetDefault(t *testing.T) {
	am := newTestAccountManager(t)
	acc1, _ := am.CreateAccount("Account1", "password123")
	_, _ = am.CreateAccount("Account2", "password123")

	t.Run("sets default to existing account", func(t *testing.T) {
		err := am.SetDefault(acc1.ID)
		if err != nil {
			t.Fatalf("SetDefault() error = %v", err)
		}
	})

	t.Run("returns error for non-existent account", func(t *testing.T) {
		err := am.SetDefault("nonexistent")
		if err == nil {
			t.Error("SetDefault() expected error for non-existent account")
		}
	})
}

func TestAccountManager_UpdateLastAccessed(t *testing.T) {
	am := newTestAccountManager(t)
	acc, _ := am.CreateAccount("TestAccount", "password123")
	originalTime := acc.LastAccessed

	t.Run("updates last accessed time", func(t *testing.T) {
		err := am.UpdateLastAccessed(acc.ID)
		if err != nil {
			t.Fatalf("UpdateLastAccessed() error = %v", err)
		}

		updated, _ := am.GetAccount(acc.ID)
		if !updated.LastAccessed.After(originalTime) {
			t.Error("UpdateLastAccessed() did not update time")
		}
	})

	t.Run("returns error for non-existent account", func(t *testing.T) {
		err := am.UpdateLastAccessed("nonexistent")
		if err == nil {
			t.Error("UpdateLastAccessed() expected error")
		}
	})
}

func TestAccountManager_GetAccountPath(t *testing.T) {
	am := newTestAccountManager(t)
	acc, _ := am.CreateAccount("TestAccount", "password123")

	t.Run("returns path for existing account", func(t *testing.T) {
		path, err := am.GetAccountPath(acc.ID)
		if err != nil {
			t.Fatalf("GetAccountPath() error = %v", err)
		}
		if path == "" {
			t.Error("GetAccountPath() returned empty path")
		}
		if !filepath.IsAbs(path) {
			t.Error("GetAccountPath() should return absolute path")
		}
	})

	t.Run("returns error for non-existent account", func(t *testing.T) {
		_, err := am.GetAccountPath("nonexistent")
		if err == nil {
			t.Error("GetAccountPath() expected error")
		}
	})
}

func TestAccountManager_GetDefaultAccount(t *testing.T) {
	t.Run("returns first account when no default", func(t *testing.T) {
		am := newTestAccountManager(t)
		acc, _ := am.CreateAccount("First", "password123")

		defaultAcc := am.GetDefaultAccount()
		if defaultAcc == nil {
			t.Fatal("GetDefaultAccount() returned nil")
		}
		if defaultAcc.ID != acc.ID {
			t.Errorf("expected default %s, got %s", acc.ID, defaultAcc.ID)
		}
	})

	t.Run("returns nil when no accounts", func(t *testing.T) {
		am := newTestAccountManager(t)
		defaultAcc := am.GetDefaultAccount()
		if defaultAcc != nil {
			t.Error("GetDefaultAccount() should return nil when no accounts")
		}
	})
}

func TestGenerateAccountID(t *testing.T) {
	t.Run("generates unique IDs", func(t *testing.T) {
		id1, err := generateAccountID()
		if err != nil {
			t.Fatalf("generateAccountID() error = %v", err)
		}
		id2, err := generateAccountID()
		if err != nil {
			t.Fatalf("generateAccountID() error = %v", err)
		}

		if id1 == "" || id2 == "" {
			t.Error("generateAccountID() returned empty ID")
		}
		if id1 == id2 {
			t.Error("generateAccountID() produced duplicate IDs")
		}
		if len(id1) < len(accountPrefix) || id1[:len(accountPrefix)] != accountPrefix {
			t.Errorf("ID %q does not start with %s", id1, accountPrefix)
		}
	})
}

// newTestAccountManager creates a new AccountManager in a temp directory
func newTestAccountManager(t *testing.T) *AccountManager {
	t.Helper()
	dir := t.TempDir()
	am, err := NewAccountManager(dir)
	if err != nil {
		t.Fatalf("NewAccountManager() error = %v", err)
	}
	return am
}
