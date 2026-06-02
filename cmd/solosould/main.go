package main

import (
	"flag"
	"log/slog"
	"os"
	"path/filepath"

	"github.com/solosoul/solosoul/core/api"
)

func main() {
	addr := flag.String("addr", ":8080", "HTTP server address")
	flag.Parse()

	// Get vault path
	vaultPath := os.Getenv("SOLOSOUL_VAULT_PATH")
	if vaultPath == "" {
		homeDir, _ := os.UserHomeDir()
		vaultPath = filepath.Join(homeDir, ".solosoul")
	}

	logger := slog.New(slog.NewJSONHandler(os.Stdout, nil))
	slog.SetDefault(logger)

	slog.Info("SoloSoul API Server starting",
		"vault_path", vaultPath,
		"addr", *addr,
	)

	server, err := api.NewHTTPServer(vaultPath)
	if err != nil {
		slog.Error("Failed to create server", "error", err)
		os.Exit(1)
	}

	slog.Info("Server listening", "addr", *addr)
	if err := server.Start(*addr); err != nil {
		slog.Error("Server error", "error", err)
		os.Exit(1)
	}
}
