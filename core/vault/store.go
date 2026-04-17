package vault

// Store interface defines the vault storage operations
type Store interface {
	// Initialize creates a new vault with the given master password
	Initialize(masterPassword string) error

	// Unlock opens the vault with the master password
	Unlock(masterPassword string) error

	// Lock closes the vault and clears keys from memory
	Lock() error

	// IsLocked returns true if the vault is locked
	IsLocked() bool

	// IsInitialized returns true if the vault has been set up
	IsInitialized() bool

	// ChangePassword changes the master password
	ChangePassword(oldPassword, newPassword string) error

	// Get derives a key for a specific field path
	Get(profileID, fieldPath string) ([]byte, error)

	// Set stores a value for a specific field path
	Set(profileID, fieldPath string, value []byte) error

	// Delete removes a field path
	Delete(profileID, fieldPath string) error

	// List returns all profile IDs
	ListProfiles() ([]string, error)

	// DeleteProfile removes an entire profile
	DeleteProfile(profileID string) error

	// Close closes the store
	Close() error

	// SetVaultPath changes the vault path (for multi-account support)
	SetVaultPath(path string) error

	// GetVaultPath returns the current vault path
	GetVaultPath() string
}

// Vault holds the encryption state
type Vault struct {
	store        Store
	derivedKey   []byte // only in memory when unlocked
	salt         []byte
	isLocked     bool
	isInitialized bool
}

// Config holds vault configuration
type Config struct {
	StoragePath string
}
