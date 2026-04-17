package vault

import (
	"encoding/json"
	"errors"
	"os"
	"path/filepath"
	"sync"

	"github.com/solosoul/solosoul/core/crypto"
)

const (
	configFileName = "config.json"
	indexFileName  = "index.db"
	profileDirName = "profiles"
	blobDirName    = "blobs"
)

// FileStore implements Store using encrypted files
type FileStore struct {
	basePath string
	vault    *Vault

	// In-memory index of profile -> field -> blob file
	index map[string]map[string]string

	mu sync.RWMutex
}

// ConfigData is stored unencrypted (salt, version, etc.)
type ConfigData struct {
	Version   string `json:"version"`
	Salt      []byte `json:"salt"`
	MasterKey []byte `json:"master_key"` // Encrypted master key for verification
}

// NewFileStore creates a new file-based store
func NewFileStore(basePath string) (*FileStore, error) {
	fs := &FileStore{
		basePath: basePath,
		index:    make(map[string]map[string]string),
		vault: &Vault{
			isLocked:      true,
			isInitialized: false,
		},
	}

	// Ensure directory structure exists
	if err := fs.ensureDirs(); err != nil {
		return nil, err
	}

	// Load existing index if vault is initialized
	if err := fs.loadConfig(); err == nil {
		fs.vault.isInitialized = true
		// Don't load salt/key until unlock
	}

	return fs, nil
}

func (fs *FileStore) ensureDirs() error {
	dirs := []string{
		fs.basePath,
		filepath.Join(fs.basePath, profileDirName),
		filepath.Join(fs.basePath, blobDirName),
	}

	for _, dir := range dirs {
		if err := os.MkdirAll(dir, 0700); err != nil {
			return err
		}
	}
	return nil
}

func (fs *FileStore) configPath() string {
	return filepath.Join(fs.basePath, configFileName)
}

func (fs *FileStore) loadConfig() error {
	data, err := os.ReadFile(fs.configPath())
	if err != nil {
		if os.IsNotExist(err) {
			return ErrVaultUninitialized
		}
		return err
	}

	var config ConfigData
	if err := json.Unmarshal(data, &config); err != nil {
		return err
	}

	fs.vault.salt = config.Salt
	return nil
}

func (fs *FileStore) saveConfig() error {
	config := ConfigData{
		Version:   "1.0",
		Salt:      fs.vault.salt,
		MasterKey: nil, // Will be set after encryption
	}

	data, err := json.Marshal(config)
	if err != nil {
		return err
	}

	return os.WriteFile(fs.configPath(), data, 0600)
}

// Initialize creates a new vault
func (fs *FileStore) Initialize(masterPassword string) error {
	fs.mu.Lock()
	defer fs.mu.Unlock()

	if fs.vault.isInitialized {
		return errors.New("vault already initialized")
	}

	// Generate salt
	salt, err := crypto.GenerateSalt()
	if err != nil {
		return err
	}

	// Derive key from password
	derivedKey, err := crypto.DeriveKey(masterPassword, salt)
	if err != nil {
		return err
	}

	// Store salt and a verification token (encrypted derived key)
	// The verification token lets us check password without storing it
	verificationBlob, err := crypto.Encrypt(derivedKey, []byte("SOLOSOUL_VAULT_V1"))
	if err != nil {
		return err
	}

	config := ConfigData{
		Version:   "1.0",
		Salt:      salt,
		MasterKey: serializeBlob(verificationBlob),
	}

	data, err := json.Marshal(config)
	if err != nil {
		return err
	}

	if err := os.WriteFile(fs.configPath(), data, 0600); err != nil {
		return err
	}

	fs.vault.salt = salt
	fs.vault.derivedKey = derivedKey
	fs.vault.isLocked = false
	fs.vault.isInitialized = true

	return nil
}

// Unlock opens the vault
func (fs *FileStore) Unlock(masterPassword string) error {
	fs.mu.Lock()
	defer fs.mu.Unlock()

	if !fs.vault.isInitialized {
		return ErrVaultUninitialized
	}

	// Load config
	data, err := os.ReadFile(fs.configPath())
	if err != nil {
		return err
	}

	var config ConfigData
	if err := json.Unmarshal(data, &config); err != nil {
		return err
	}

	// Derive key from password
	derivedKey, err := crypto.DeriveKey(masterPassword, config.Salt)
	if err != nil {
		return err
	}

	// Verify password by decrypting verification token
	if config.MasterKey != nil {
		blob := deserializeBlob(config.MasterKey)
		plaintext, err := crypto.Decrypt(derivedKey, blob)
		if err != nil || string(plaintext) != "SOLOSOUL_VAULT_V1" {
			return ErrInvalidPassword
		}
	}

	fs.vault.salt = config.Salt
	fs.vault.derivedKey = derivedKey
	fs.vault.isLocked = false

	// Load index
	fs.loadIndex()

	return nil
}

// Lock closes the vault
func (fs *FileStore) Lock() error {
	fs.mu.Lock()
	defer fs.mu.Unlock()

	if fs.vault.derivedKey != nil {
		crypto.SecureWipe(fs.vault.derivedKey)
		fs.vault.derivedKey = nil
	}
	fs.vault.isLocked = true
	fs.index = make(map[string]map[string]string)

	return nil
}

func (fs *FileStore) IsLocked() bool {
	fs.mu.RLock()
	defer fs.mu.RUnlock()
	return fs.vault.isLocked
}

func (fs *FileStore) IsInitialized() bool {
	fs.mu.RLock()
	defer fs.mu.RUnlock()
	return fs.vault.isInitialized
}

func (fs *FileStore) ChangePassword(oldPassword, newPassword string) error {
	// Verify old password first
	if err := fs.Unlock(oldPassword); err != nil {
		return err
	}

	// Generate new salt and derive new key
	newSalt, err := crypto.GenerateSalt()
	if err != nil {
		return err
	}

	newKey, err := crypto.DeriveKey(newPassword, newSalt)
	if err != nil {
		return err
	}

	// Re-encrypt all blobs with new key
	oldKey := fs.vault.derivedKey
	for profileID, fields := range fs.index {
		for fieldPath, blobPath := range fields {
			ciphertext, err := os.ReadFile(blobPath)
			if err != nil {
				return err
			}
			blob := deserializeBlob(ciphertext)
			plaintext, err := crypto.Decrypt(oldKey, blob)
			if err != nil {
				return err
			}
			newBlob, err := crypto.Encrypt(newKey, plaintext)
			if err != nil {
				return err
			}
			if err := os.WriteFile(blobPath, serializeBlob(newBlob), 0600); err != nil {
				return err
			}
			// Update index to reflect new key (path stays same)
			_ = profileID
			_ = fieldPath
		}
	}

	// Update config
	config := ConfigData{
		Version:   "1.0",
		Salt:      newSalt,
		MasterKey: nil,
	}

	verificationBlob, err := crypto.Encrypt(newKey, []byte("SOLOSOUL_VAULT_V1"))
	if err != nil {
		return err
	}
	config.MasterKey = serializeBlob(verificationBlob)

	data, err := json.Marshal(config)
	if err != nil {
		return err
	}

	if err := os.WriteFile(fs.configPath(), data, 0600); err != nil {
		return err
	}

	fs.vault.salt = newSalt
	fs.vault.derivedKey = newKey

	return nil
}

func (fs *FileStore) Get(profileID, fieldPath string) ([]byte, error) {
	fs.mu.RLock()
	defer fs.mu.RUnlock()

	if fs.vault.isLocked {
		return nil, ErrVaultLocked
	}

	blobPath, ok := fs.index[profileID][fieldPath]
	if !ok {
		return nil, nil // Not found, not an error
	}

	ciphertext, err := os.ReadFile(blobPath)
	if err != nil {
		return nil, err
	}

	blob := deserializeBlob(ciphertext)
	return crypto.Decrypt(fs.vault.derivedKey, blob)
}

func (fs *FileStore) Set(profileID, fieldPath string, value []byte) error {
	fs.mu.Lock()
	defer fs.mu.Unlock()

	if fs.vault.isLocked {
		return ErrVaultLocked
	}

	blob, err := crypto.Encrypt(fs.vault.derivedKey, value)
	if err != nil {
		return err
	}

	// Ensure profile dir exists
	profileDir := filepath.Join(fs.basePath, profileDirName, profileID)
	if err := os.MkdirAll(profileDir, 0700); err != nil {
		return err
	}

	// Write blob
	blobPath := filepath.Join(profileDir, cryptoHash(fieldPath)+".enc")
	if err := os.WriteFile(blobPath, serializeBlob(blob), 0600); err != nil {
		return err
	}

	// Update index
	if fs.index[profileID] == nil {
		fs.index[profileID] = make(map[string]string)
	}
	fs.index[profileID][fieldPath] = blobPath

	return fs.saveIndex()
}

func (fs *FileStore) Delete(profileID, fieldPath string) error {
	fs.mu.Lock()
	defer fs.mu.Unlock()

	if fs.vault.isLocked {
		return ErrVaultLocked
	}

	blobPath, ok := fs.index[profileID][fieldPath]
	if !ok {
		return nil
	}

	if err := os.Remove(blobPath); err != nil && !os.IsNotExist(err) {
		return err
	}

	delete(fs.index[profileID], fieldPath)
	return fs.saveIndex()
}

func (fs *FileStore) ListProfiles() ([]string, error) {
	fs.mu.RLock()
	defer fs.mu.RUnlock()

	profiles := make([]string, 0, len(fs.index))
	for profileID := range fs.index {
		profiles = append(profiles, profileID)
	}
	return profiles, nil
}

func (fs *FileStore) DeleteProfile(profileID string) error {
	fs.mu.Lock()
	defer fs.mu.Unlock()

	if fs.vault.isLocked {
		return ErrVaultLocked
	}

	// Delete all blobs
	profileDir := filepath.Join(fs.basePath, profileDirName, profileID)
	if err := os.RemoveAll(profileDir); err != nil {
		return err
	}

	delete(fs.index, profileID)
	return fs.saveIndex()
}

func (fs *FileStore) Close() error {
	return fs.Lock()
}

func (fs *FileStore) SetVaultPath(path string) error {
	fs.mu.Lock()
	defer fs.mu.Unlock()

	// Lock current vault
	if fs.vault.derivedKey != nil {
		crypto.SecureWipe(fs.vault.derivedKey)
		fs.vault.derivedKey = nil
	}
	fs.vault.isLocked = true

	// Update base path
	fs.basePath = path

	// Reset index
	fs.index = make(map[string]map[string]string)

	// Re-initialize vault state
	fs.vault.salt = nil
	fs.vault.isInitialized = false

	// Ensure directory exists
	if err := fs.ensureDirs(); err != nil {
		return err
	}

	// Try to load existing config
	if err := fs.loadConfig(); err == nil {
		fs.vault.isInitialized = true
	}

	// Load index so profile data is accessible
	if err := fs.loadIndex(); err != nil {
		return err
	}

	return nil
}

func (fs *FileStore) GetVaultPath() string {
	fs.mu.RLock()
	defer fs.mu.RUnlock()
	return fs.basePath
}

func (fs *FileStore) loadIndex() error {
	indexPath := filepath.Join(fs.basePath, indexFileName)
	data, err := os.ReadFile(indexPath)
	if err != nil {
		if os.IsNotExist(err) {
			fs.index = make(map[string]map[string]string)
			return nil
		}
		return err
	}

	return json.Unmarshal(data, &fs.index)
}

func (fs *FileStore) saveIndex() error {
	data, err := json.Marshal(fs.index)
	if err != nil {
		return err
	}

	indexPath := filepath.Join(fs.basePath, indexFileName)
	return os.WriteFile(indexPath, data, 0600)
}

// Helper functions

func serializeBlob(blob *crypto.EncryptedBlob) []byte {
	result := make([]byte, len(blob.Nonce)+len(blob.Ciphertext))
	copy(result, blob.Nonce[:])
	copy(result[len(blob.Nonce):], blob.Ciphertext)
	return result
}

func deserializeBlob(data []byte) *crypto.EncryptedBlob {
	nonceLen := crypto.AES256GCMNonceLen
	if len(data) < nonceLen {
		return &crypto.EncryptedBlob{}
	}
	blob := &crypto.EncryptedBlob{
		Ciphertext: data[nonceLen:],
	}
	copy(blob.Nonce[:], data[:nonceLen])
	return blob
}

func cryptoHash(s string) string {
	// Simple hash for filename - not cryptographic
	h := uint32(2166136261)
	for _, c := range s {
		h ^= uint32(c)
		h *= 16777619
	}
	return string(rune(h))
}

// Errors
var (
	ErrVaultUninitialized = errors.New("vault not initialized")
	ErrVaultLocked        = errors.New("vault is locked")
	ErrInvalidPassword    = errors.New("invalid password")
)
