package vault

import (
	"os"
	"path/filepath"
	"testing"

	"github.com/solosoul/solosoul/core/crypto"
)

func TestNewFileStore(t *testing.T) {
	// Use a unique directory to avoid test interference
	tmpDir := t.TempDir()

	// Ensure the directory is truly empty
	configPath := filepath.Join(tmpDir, configFileName)
	if _, err := os.Stat(configPath); err == nil {
		t.Skip("Temp directory already has config file, skipping")
	}

	fs, err := NewFileStore(tmpDir)
	if err != nil {
		t.Fatalf("NewFileStore(%q) failed: %v", tmpDir, err)
	}

	if fs == nil {
		t.Fatal("NewFileStore returned nil")
	}

	if fs.IsInitialized() {
		t.Error("NewFileStore should not be initialized for new directory")
	}

	if !fs.IsLocked() {
		t.Error("NewFileStore should be locked by default")
	}
}

func TestFileStore_Initialize(t *testing.T) {
	tmpDir := t.TempDir()

	fs, err := NewFileStore(tmpDir)
	if err != nil {
		t.Fatalf("NewFileStore failed: %v", err)
	}

	password := "testpassword123"

	err = fs.Initialize(password)
	if err != nil {
		t.Fatalf("Initialize() failed: %v", err)
	}

	if !fs.IsInitialized() {
		t.Error("After Initialize(), IsInitialized() should be true")
	}

	if fs.IsLocked() {
		t.Error("After Initialize(), IsLocked() should be false")
	}

	// Verify config file was created
	configPath := filepath.Join(tmpDir, configFileName)
	if _, err := os.Stat(configPath); os.IsNotExist(err) {
		t.Error("Config file should be created after Initialize()")
	}
}

func TestFileStore_Initialize_AlreadyInitialized(t *testing.T) {
	tmpDir := t.TempDir()

	fs, err := NewFileStore(tmpDir)
	if err != nil {
		t.Fatalf("NewFileStore failed: %v", err)
	}

	password := "testpassword123"

	// First initialization
	if err := fs.Initialize(password); err != nil {
		t.Fatalf("First Initialize() failed: %v", err)
	}

	// Second initialization should fail
	if err := fs.Initialize(password); err == nil {
		t.Error("Second Initialize() should fail on already initialized vault")
	}
}

func TestFileStore_Unlock(t *testing.T) {
	tmpDir := t.TempDir()

	fs, err := NewFileStore(tmpDir)
	if err != nil {
		t.Fatalf("NewFileStore failed: %v", err)
	}

	password := "testpassword123"

	// Initialize first
	if err := fs.Initialize(password); err != nil {
		t.Fatalf("Initialize() failed: %v", err)
	}

	// Lock it
	if err := fs.Lock(); err != nil {
		t.Fatalf("Lock() failed: %v", err)
	}

	// Unlock with correct password
	if err := fs.Unlock(password); err != nil {
		t.Fatalf("Unlock() with correct password failed: %v", err)
	}

	if fs.IsLocked() {
		t.Error("After Unlock(), IsLocked() should be false")
	}
}

func TestFileStore_Unlock_WrongPassword(t *testing.T) {
	tmpDir := t.TempDir()

	fs, err := NewFileStore(tmpDir)
	if err != nil {
		t.Fatalf("NewFileStore failed: %v", err)
	}

	password := "testpassword123"
	wrongPassword := "wrongpassword"

	// Initialize first
	if err := fs.Initialize(password); err != nil {
		t.Fatalf("Initialize() failed: %v", err)
	}

	// Lock it
	if err := fs.Lock(); err != nil {
		t.Fatalf("Lock() failed: %v", err)
	}

	// Unlock with wrong password should fail
	if err := fs.Unlock(wrongPassword); err != ErrInvalidPassword {
		t.Errorf("Unlock() with wrong password error = %v, want %v", err, ErrInvalidPassword)
	}
}

func TestFileStore_Unlock_NotInitialized(t *testing.T) {
	tmpDir := t.TempDir()

	// Create store but directory doesn't exist yet
	os.RemoveAll(tmpDir)

	fs, err := NewFileStore(tmpDir)
	if err != nil {
		t.Fatalf("NewFileStore failed: %v", err)
	}

	// Try to unlock without initialization
	if err := fs.Unlock("anypassword"); err != ErrVaultUninitialized {
		t.Errorf("Unlock() on uninitialized vault error = %v, want %v", err, ErrVaultUninitialized)
	}
}

func TestFileStore_Lock(t *testing.T) {
	tmpDir := t.TempDir()

	fs, err := NewFileStore(tmpDir)
	if err != nil {
		t.Fatalf("NewFileStore failed: %v", err)
	}

	password := "testpassword123"

	// Initialize and unlock
	if err := fs.Initialize(password); err != nil {
		t.Fatalf("Initialize() failed: %v", err)
	}

	if err := fs.Lock(); err != nil {
		t.Fatalf("Lock() failed: %v", err)
	}

	if !fs.IsLocked() {
		t.Error("After Lock(), IsLocked() should be true")
	}
}

func TestFileStore_ChangePassword(t *testing.T) {
	tmpDir := t.TempDir()

	fs, err := NewFileStore(tmpDir)
	if err != nil {
		t.Fatalf("NewFileStore failed: %v", err)
	}

	oldPassword := "oldpassword123"
	newPassword := "newpassword456"

	// Initialize
	if err := fs.Initialize(oldPassword); err != nil {
		t.Fatalf("Initialize() failed: %v", err)
	}

	// Change password
	if err := fs.ChangePassword(oldPassword, newPassword); err != nil {
		t.Fatalf("ChangePassword() failed: %v", err)
	}

	// Lock and try to unlock with old password
	if err := fs.Lock(); err != nil {
		t.Fatalf("Lock() failed: %v", err)
	}

	if err := fs.Unlock(oldPassword); err != ErrInvalidPassword {
		t.Errorf("Unlock() with old password error = %v, want %v", err, ErrInvalidPassword)
	}

	// Unlock with new password should work
	if err := fs.Unlock(newPassword); err != nil {
		t.Errorf("Unlock() with new password failed: %v", err)
	}
}

func TestFileStore_Get_Set(t *testing.T) {
	tmpDir := t.TempDir()

	fs, err := NewFileStore(tmpDir)
	if err != nil {
		t.Fatalf("NewFileStore failed: %v", err)
	}

	password := "testpassword123"

	if err := fs.Initialize(password); err != nil {
		t.Fatalf("Initialize() failed: %v", err)
	}

	profileID := "test-profile"
	fieldPath := "identity.full_name"
	value := []byte("John Doe")

	// Set a value
	if err := fs.Set(profileID, fieldPath, value); err != nil {
		t.Fatalf("Set() failed: %v", err)
	}

	// Get the value back
	got, err := fs.Get(profileID, fieldPath)
	if err != nil {
		t.Fatalf("Get() failed: %v", err)
	}

	if string(got) != string(value) {
		t.Errorf("Get() = %q, want %q", string(got), string(value))
	}
}

func TestFileStore_Get_Locked(t *testing.T) {
	tmpDir := t.TempDir()

	fs, err := NewFileStore(tmpDir)
	if err != nil {
		t.Fatalf("NewFileStore failed: %v", err)
	}

	password := "testpassword123"

	if err := fs.Initialize(password); err != nil {
		t.Fatalf("Initialize() failed: %v", err)
	}

	if err := fs.Lock(); err != nil {
		t.Fatalf("Lock() failed: %v", err)
	}

	_, err = fs.Get("profile", "field")
	if err != ErrVaultLocked {
		t.Errorf("Get() on locked vault error = %v, want %v", err, ErrVaultLocked)
	}
}

func TestFileStore_Set_Locked(t *testing.T) {
	tmpDir := t.TempDir()

	fs, err := NewFileStore(tmpDir)
	if err != nil {
		t.Fatalf("NewFileStore failed: %v", err)
	}

	password := "testpassword123"

	if err := fs.Initialize(password); err != nil {
		t.Fatalf("Initialize() failed: %v", err)
	}

	if err := fs.Lock(); err != nil {
		t.Fatalf("Lock() failed: %v", err)
	}

	err = fs.Set("profile", "field", []byte("value"))
	if err != ErrVaultLocked {
		t.Errorf("Set() on locked vault error = %v, want %v", err, ErrVaultLocked)
	}
}

func TestFileStore_Delete(t *testing.T) {
	tmpDir := t.TempDir()

	fs, err := NewFileStore(tmpDir)
	if err != nil {
		t.Fatalf("NewFileStore failed: %v", err)
	}

	password := "testpassword123"

	if err := fs.Initialize(password); err != nil {
		t.Fatalf("Initialize() failed: %v", err)
	}

	profileID := "test-profile"
	fieldPath := "identity.full_name"
	value := []byte("John Doe")

	// Set a value
	if err := fs.Set(profileID, fieldPath, value); err != nil {
		t.Fatalf("Set() failed: %v", err)
	}

	// Delete the value
	if err := fs.Delete(profileID, fieldPath); err != nil {
		t.Fatalf("Delete() failed: %v", err)
	}

	// Get should return nil (not found)
	got, err := fs.Get(profileID, fieldPath)
	if err != nil {
		t.Fatalf("Get() failed: %v", err)
	}
	if got != nil {
		t.Errorf("Get() after Delete() = %v, want nil", got)
	}
}

func TestFileStore_DeleteProfile(t *testing.T) {
	tmpDir := t.TempDir()

	fs, err := NewFileStore(tmpDir)
	if err != nil {
		t.Fatalf("NewFileStore failed: %v", err)
	}

	password := "testpassword123"

	if err := fs.Initialize(password); err != nil {
		t.Fatalf("Initialize() failed: %v", err)
	}

	profileID := "test-profile"
	fieldPath := "identity.full_name"
	value := []byte("John Doe")

	// Set a value
	if err := fs.Set(profileID, fieldPath, value); err != nil {
		t.Fatalf("Set() failed: %v", err)
	}

	// Delete the profile
	if err := fs.DeleteProfile(profileID); err != nil {
		t.Fatalf("DeleteProfile() failed: %v", err)
	}

	// Get should return nil (not found)
	got, err := fs.Get(profileID, fieldPath)
	if err != nil {
		t.Fatalf("Get() failed: %v", err)
	}
	if got != nil {
		t.Errorf("Get() after DeleteProfile() = %v, want nil", got)
	}
}

func TestFileStore_ListProfiles(t *testing.T) {
	tmpDir := t.TempDir()

	fs, err := NewFileStore(tmpDir)
	if err != nil {
		t.Fatalf("NewFileStore failed: %v", err)
	}

	password := "testpassword123"

	if err := fs.Initialize(password); err != nil {
		t.Fatalf("Initialize() failed: %v", err)
	}

	profiles := []string{"profile1", "profile2", "profile3"}

	for _, profileID := range profiles {
		if err := fs.Set(profileID, "field", []byte("value")); err != nil {
			t.Fatalf("Set() for %s failed: %v", profileID, err)
		}
	}

	listed, err := fs.ListProfiles()
	if err != nil {
		t.Fatalf("ListProfiles() failed: %v", err)
	}

	if len(listed) != len(profiles) {
		t.Errorf("ListProfiles() returned %d profiles, want %d", len(listed), len(profiles))
	}
}

func TestFileStore_Close(t *testing.T) {
	tmpDir := t.TempDir()

	fs, err := NewFileStore(tmpDir)
	if err != nil {
		t.Fatalf("NewFileStore failed: %v", err)
	}

	password := "testpassword123"

	if err := fs.Initialize(password); err != nil {
		t.Fatalf("Initialize() failed: %v", err)
	}

	if err := fs.Close(); err != nil {
		t.Fatalf("Close() failed: %v", err)
	}

	// After close, vault should be locked
	if !fs.IsLocked() {
		t.Error("After Close(), IsLocked() should be true")
	}
}

func TestFileStore_EnsureDirs(t *testing.T) {
	tmpDir := t.TempDir()

	fs, err := NewFileStore(tmpDir)
	if err != nil {
		t.Fatalf("NewFileStore failed: %v", err)
	}

	// Test ensureDirs creates directories
	if err := fs.ensureDirs(); err != nil {
		t.Fatalf("ensureDirs() failed: %v", err)
	}

	// Verify directories exist
	dirs := []string{
		tmpDir,
		filepath.Join(tmpDir, profileDirName),
		filepath.Join(tmpDir, blobDirName),
	}

	for _, dir := range dirs {
		info, err := os.Stat(dir)
		if err != nil {
			t.Errorf("Directory %q does not exist: %v", dir, err)
		}
		if !info.IsDir() {
			t.Errorf("Path %q exists but is not a directory", dir)
		}
	}
}

func TestSerializeDeserializeBlob(t *testing.T) {
	// Test serialization and deserialization of blob format
	// This test doesn't need actual encryption, just verifies the format works

	// Create a fake blob for serialization testing
	blob := &crypto.EncryptedBlob{}
	for i := range blob.Nonce {
		blob.Nonce[i] = byte(i)
	}
	blob.Ciphertext = []byte("test ciphertext data")

	serialized := serializeBlob(blob)

	if len(serialized) == 0 {
		t.Error("serializeBlob() returned empty result")
	}

	if len(serialized) != len(blob.Nonce)+len(blob.Ciphertext) {
		t.Errorf("serialized length = %d, want %d", len(serialized), len(blob.Nonce)+len(blob.Ciphertext))
	}

	deserialized := deserializeBlob(serialized)

	if deserialized == nil {
		t.Fatal("deserializeBlob() returned nil")
	}

	// Verify the nonce was correctly deserialized
	for i := range blob.Nonce {
		if deserialized.Nonce[i] != blob.Nonce[i] {
			t.Errorf("Nonce[%d] = %d, want %d", i, deserialized.Nonce[i], blob.Nonce[i])
		}
	}

	// Verify the ciphertext was correctly deserialized
	if string(deserialized.Ciphertext) != string(blob.Ciphertext) {
		t.Errorf("Ciphertext = %q, want %q", string(deserialized.Ciphertext), string(blob.Ciphertext))
	}
}

func TestDeserializeBlob_TooShort(t *testing.T) {
	// Test deserialization with data too short to contain nonce
	result := deserializeBlob([]byte("tooshort"))
	if result == nil {
		t.Fatal("deserializeBlob() should return empty blob, not nil")
	}
	if len(result.Ciphertext) != 0 {
		t.Errorf("Ciphertext len = %d, want 0", len(result.Ciphertext))
	}
}

func TestDeserializeBlob_Empty(t *testing.T) {
	result := deserializeBlob([]byte{})
	if result == nil {
		t.Fatal("deserializeBlob() should return empty blob, not nil")
	}
}

func TestCryptoHash(t *testing.T) {
	tests := []struct {
		input    string
		wantSame bool // Whether same input should produce same output
	}{
		{"test1", true},
		{"test2", true},
	}

	for _, tt := range tests {
		t.Run(tt.input, func(t *testing.T) {
			h1 := cryptoHash(tt.input)
			h2 := cryptoHash(tt.input)

			if h1 != h2 {
				t.Errorf("cryptoHash(%q) produced different results: %q vs %q", tt.input, h1, h2)
			}
		})
	}

	// Same input should produce same output (deterministic)
	h1 := cryptoHash("input1")
	h2 := cryptoHash("input1")
	if h1 != h2 {
		t.Error("Same input should produce same hash")
	}
}

func TestFileStore_SetVaultPath(t *testing.T) {
	t.Run("changes vault path and resets state", func(t *testing.T) {
		dir1 := t.TempDir()
		dir2 := t.TempDir()

		fs, err := NewFileStore(dir1)
		if err != nil {
			t.Fatalf("NewFileStore failed: %v", err)
		}

		// Initialize and unlock first vault
		password := "testpassword123"
		if err := fs.Initialize(password); err != nil {
			t.Fatalf("Initialize() failed: %v", err)
		}

		// Change to second path
		if err := fs.SetVaultPath(dir2); err != nil {
			t.Fatalf("SetVaultPath() failed: %v", err)
		}

		// Should be locked after path change
		if !fs.IsLocked() {
			t.Error("SetVaultPath() should lock the vault")
		}

		// Path should be updated
		if fs.GetVaultPath() != dir2 {
			t.Errorf("GetVaultPath() = %q, want %q", fs.GetVaultPath(), dir2)
		}
	})

	t.Run("switches to initialized vault", func(t *testing.T) {
		dir1 := t.TempDir()
		dir2 := t.TempDir()

		// Setup two initialized vaults
		fs1, _ := NewFileStore(dir1)
		fs1.Initialize("password1")

		fs2, _ := NewFileStore(dir2)
		fs2.Initialize("password2")

		// Switch fs1 to dir2
		if err := fs1.SetVaultPath(dir2); err != nil {
			t.Fatalf("SetVaultPath() failed: %v", err)
		}

		// Should detect initialization
		if !fs1.IsInitialized() {
			t.Error("SetVaultPath() should detect existing initialized vault")
		}
	})
}

func TestFileStore_GetVaultPath(t *testing.T) {
	t.Run("returns initial path", func(t *testing.T) {
		dir := t.TempDir()
		fs, _ := NewFileStore(dir)

		path := fs.GetVaultPath()
		if path != dir {
			t.Errorf("GetVaultPath() = %q, want %q", path, dir)
		}
	})

	t.Run("returns updated path after SetVaultPath", func(t *testing.T) {
		dir1 := t.TempDir()
		dir2 := t.TempDir()
		fs, _ := NewFileStore(dir1)

		fs.SetVaultPath(dir2)
		if fs.GetVaultPath() != dir2 {
			t.Errorf("GetVaultPath() after SetVaultPath = %q, want %q", fs.GetVaultPath(), dir2)
		}
	})
}
