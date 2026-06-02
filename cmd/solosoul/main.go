package main

import (
	"bufio"
	"encoding/json"
	"fmt"
	"os"
	"os/user"
	"path/filepath"
	"strings"
	"syscall"

	"github.com/solosoul/solosoul/core/schema"
	"github.com/solosoul/solosoul/core/vault"
	"golang.org/x/term"
)

func main() {
	// Get default vault path
	homeDir, _ := os.UserHomeDir()
	defaultVaultPath := filepath.Join(homeDir, ".solosoul")

	// Parse command line
	args := os.Args[1:]
	if len(args) == 0 {
		printUsage()
		os.Exit(1)
	}

	command := args[0]

	// Get vault path from env or default
	vaultPath := os.Getenv("SOLOSOUL_VAULT_PATH")
	if vaultPath == "" {
		vaultPath = defaultVaultPath
	}

	switch command {
	case "init":
		cmdInit(vaultPath, args[1:])
	case "unlock":
		cmdUnlock(vaultPath, args[1:])
	case "lock":
		cmdLock(vaultPath)
	case "status":
		cmdStatus(vaultPath)
	case "profile":
		cmdProfile(vaultPath, args[1:])
	case "set":
		cmdSet(vaultPath, args[1:])
	default:
		fmt.Printf("Unknown command: %s\n", command)
		printUsage()
		os.Exit(1)
	}
}

func printUsage() {
	fmt.Print(`SoloSoul - Local Digital Twin Engine

Usage:
  solosoul <command> [options]

Commands:
  init                    Initialize a new vault
  unlock                  Unlock the vault
  lock                    Lock the vault
  status                  Show vault status
  profile <action>         Manage profiles
  set <profile> <field> <value>  Set a field value

Environment:
  SOLOSOUL_VAULT_PATH     Override vault location (default: ~/.solosoul)

Examples:
  solosoul init
  solosoul unlock
  solosoul profile list
  solosoul set default identity.full_name "John Doe"
`)
}

func cmdInit(vaultPath string, args []string) {
	store, err := vault.NewFileStore(vaultPath)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error creating vault: %v\n", err)
		os.Exit(1)
	}

	// Check if already initialized
	if store.IsInitialized() {
		fmt.Println("Vault already initialized. Use 'unlock' to open it.")
		os.Exit(1)
	}

	fmt.Print("Enter master password: ")
	password, err := readPassword()
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error reading password: %v\n", err)
		os.Exit(1)
	}

	if len(password) < 8 {
		fmt.Println("Password must be at least 8 characters.")
		os.Exit(1)
	}

	fmt.Print("Confirm master password: ")
	confirm, err := readPassword()
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error reading password: %v\n", err)
		os.Exit(1)
	}

	if string(password) != string(confirm) {
		fmt.Println("Passwords do not match.")
		os.Exit(1)
	}

	if err := store.Initialize(string(password)); err != nil {
		fmt.Fprintf(os.Stderr, "Error initializing vault: %v\n", err)
		os.Exit(1)
	}

	fmt.Println("Vault initialized successfully.")
	fmt.Printf("Vault location: %s\n", vaultPath)
	fmt.Println("IMPORTANT: Remember your master password! There is no recovery option.")
}

func cmdUnlock(vaultPath string, args []string) {
	store, err := vault.NewFileStore(vaultPath)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error opening vault: %v\n", err)
		os.Exit(1)
	}

	if !store.IsInitialized() {
		fmt.Println("Vault not initialized. Run 'solosoul init' first.")
		os.Exit(1)
	}

	if !store.IsLocked() {
		fmt.Println("Vault is already unlocked.")
		os.Exit(0)
	}

	fmt.Print("Enter master password: ")
	password, err := readPassword()
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error reading password: %v\n", err)
		os.Exit(1)
	}

	if err := store.Unlock(string(password)); err != nil {
		fmt.Fprintf(os.Stderr, "Error unlocking vault: %v\n", err)
		os.Exit(1)
	}

	fmt.Println("Vault unlocked successfully.")
}

func cmdLock(vaultPath string) {
	store, err := vault.NewFileStore(vaultPath)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error opening vault: %v\n", err)
		os.Exit(1)
	}

	if store.IsLocked() {
		fmt.Println("Vault is already locked.")
		os.Exit(0)
	}

	if err := store.Lock(); err != nil {
		fmt.Fprintf(os.Stderr, "Error locking vault: %v\n", err)
		os.Exit(1)
	}

	fmt.Println("Vault locked.")
}

func cmdStatus(vaultPath string) {
	store, err := vault.NewFileStore(vaultPath)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error opening vault: %v\n", err)
		os.Exit(1)
	}

	fmt.Printf("Vault path: %s\n", vaultPath)
	fmt.Printf("Initialized: %v\n", store.IsInitialized())
	fmt.Printf("Locked: %v\n", store.IsLocked())

	if store.IsInitialized() && !store.IsLocked() {
		profiles, _ := store.ListProfiles()
		fmt.Printf("Profiles: %d\n", len(profiles))
	}
}

func cmdProfile(vaultPath string, args []string) {
	if len(args) == 0 {
		printProfileUsage()
		os.Exit(1)
	}

	action := args[0]

	store, err := vault.NewFileStore(vaultPath)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error opening vault: %v\n", err)
		os.Exit(1)
	}

	switch action {
	case "list":
		if !store.IsInitialized() {
			fmt.Println("Vault not initialized.")
			os.Exit(1)
		}
		if store.IsLocked() {
			fmt.Println("Vault is locked. Unlock first.")
			os.Exit(1)
		}

		profiles, err := store.ListProfiles()
		if err != nil {
			fmt.Fprintf(os.Stderr, "Error listing profiles: %v\n", err)
			os.Exit(1)
		}

		if len(profiles) == 0 {
			fmt.Println("No profiles found.")
		} else {
			fmt.Println("Profiles:")
			for _, p := range profiles {
				fmt.Printf("  - %s\n", p)
			}
		}

	case "create":
		if len(args) < 2 {
			fmt.Println("Usage: solosoul profile create <profile_id>")
			os.Exit(1)
		}
		profileID := args[1]

		if store.IsLocked() {
			fmt.Println("Vault is locked. Unlock first.")
			os.Exit(1)
		}

		profile := schema.NewProfile(profileID)
		data, err := json.MarshalIndent(profile, "", "  ")
		if err != nil {
			fmt.Fprintf(os.Stderr, "Error creating profile: %v\n", err)
			os.Exit(1)
		}

		if err := store.Set(profileID, "_profile", data); err != nil {
			fmt.Fprintf(os.Stderr, "Error saving profile: %v\n", err)
			os.Exit(1)
		}

		fmt.Printf("Profile '%s' created.\n", profileID)

	case "get":
		if len(args) < 2 {
			fmt.Println("Usage: solosoul profile get <profile_id>")
			os.Exit(1)
		}
		profileID := args[1]

		if store.IsLocked() {
			fmt.Println("Vault is locked. Unlock first.")
			os.Exit(1)
		}

		data, err := store.Get(profileID, "_profile")
		if err != nil {
			fmt.Fprintf(os.Stderr, "Error reading profile: %v\n", err)
			os.Exit(1)
		}

		if data == nil {
			fmt.Printf("Profile '%s' not found.\n", profileID)
			os.Exit(1)
		}

		fmt.Println(string(data))

	default:
		fmt.Printf("Unknown action: %s\n", action)
		printProfileUsage()
		os.Exit(1)
	}
}

func printProfileUsage() {
	fmt.Print(`Profile commands:
  list                    List all profiles
  create <id>            Create a new profile
  get <id>               Get profile details
`)
}

func cmdSet(vaultPath string, args []string) {
	if len(args) < 3 {
		fmt.Println("Usage: solosoul set <profile_id> <field_path> <value>")
		os.Exit(1)
	}

	profileID := args[0]
	fieldPath := args[1]
	value := strings.Join(args[2:], " ")

	store, err := vault.NewFileStore(vaultPath)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error opening vault: %v\n", err)
		os.Exit(1)
	}

	if store.IsLocked() {
		fmt.Println("Vault is locked. Unlock first.")
		os.Exit(1)
	}

	if err := store.Set(profileID, fieldPath, []byte(value)); err != nil {
		fmt.Fprintf(os.Stderr, "Error setting field: %v\n", err)
		os.Exit(1)
	}

	fmt.Printf("Set %s.%s = %s\n", profileID, fieldPath, value)
}

func readPassword() ([]byte, error) {
	// Check if stdin is a terminal
	if term.IsTerminal(int(syscall.Stdin)) {
		return term.ReadPassword(int(syscall.Stdin))
	}
	// Fallback to buffered reader
	reader := bufio.NewReader(os.Stdin)
	line, err := reader.ReadString('\n')
	if err != nil {
		return nil, err
	}
	return []byte(strings.TrimSpace(line)), nil
}

// getCurrentUser returns the current username
func getCurrentUser() string {
	if u, err := user.Current(); err == nil {
		return u.Username
	}
	return "unknown"
}
