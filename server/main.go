package main

import (
	"context"
	"crypto/rand"
	"crypto/sha256"
	"encoding/base64"
	"encoding/json"
	"log"
	"net/http"
	"os"
	"os/signal"
	"path/filepath"
	"regexp"
	"syscall"
	"time"

	"github.com/joho/godotenv"
	"golang.org/x/crypto/bcrypt"
	"golang.org/x/time/rate"
)

type User struct {
	Email     string    `json:"email"`
	Password  string    `json:"-"`
	Token     string    `json:"token,omitempty"`
	CreatedAt time.Time `json:"created_at"`
}

type SyncData struct {
	Files    map[string]string `json:"files"`
	Packages []Package         `json:"packages"`
}

type Package struct {
	Name      string  `json:"name"`
	Version   *string `json:"version,omitempty"`
	Installed bool    `json:"installed"`
}

type LoginRequest struct {
	Email    string `json:"email"`
	Password string `json:"password"`
}

type RegisterRequest struct {
	Email    string `json:"email"`
	Password string `json:"password"`
}

const (
	dataDir      = "/opt/kiwi/data"
	usersDir     = "/opt/kiwi/users"
	authTokenEnv = "KIWI_AUTH_TOKEN"
)

var (
	limiter    = rate.NewLimiter(rate.Every(time.Second), 10)
	emailRegex = regexp.MustCompile(`^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$`)
)

func generateToken() (string, error) {
	b := make([]byte, 32)
	if _, err := rand.Read(b); err != nil {
		return "", err
	}
	return base64.URLEncoding.EncodeToString(b), nil
}

func getUserPath(email string) string {
	hash := sha256.Sum256([]byte(email))
	userHash := base64.URLEncoding.EncodeToString(hash[:])
	return filepath.Join(usersDir, userHash+".json")
}

func getUserDataDir(email string) string {
	hash := sha256.Sum256([]byte(email))
	userHash := base64.URLEncoding.EncodeToString(hash[:])
	return filepath.Join(dataDir, userHash)
}

func loadUser(email string) (*User, error) {
	data, err := os.ReadFile(getUserPath(email))
	if err != nil {
		return nil, err
	}
	var user User
	if err := json.Unmarshal(data, &user); err != nil {
		return nil, err
	}
	return &user, nil
}

func saveUser(user *User) error {
	data, err := json.MarshalIndent(user, "", "  ")
	if err != nil {
		return err
	}
	return os.WriteFile(getUserPath(user.Email), data, 0600)
}

func authMiddleware(next http.HandlerFunc) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		auth := r.Header.Get("Authorization")
		if auth == "" {
			http.Error(w, "Unauthorized - No token provided", http.StatusUnauthorized)
			return
		}

		// Remove "Bearer " prefix if present
		if len(auth) > 7 && auth[:7] == "Bearer " {
			auth = auth[7:]
		}

		// First check if it's an admin token
		if auth == os.Getenv(authTokenEnv) {
			r.Header.Set("X-User-Role", "admin")
			next.ServeHTTP(w, r)
			return
		}

		// Try to find user by token
		files, err := os.ReadDir(usersDir)
		if err != nil {
			http.Error(w, "Internal server error", http.StatusInternalServerError)
			return
		}

		var foundUser *User
		for _, file := range files {
			if file.IsDir() {
				continue
			}
			data, err := os.ReadFile(filepath.Join(usersDir, file.Name()))
			if err != nil {
				continue
			}
			var user User
			if err := json.Unmarshal(data, &user); err != nil {
				continue
			}
			if user.Token == auth {
				foundUser = &user
				break
			}
		}

		if foundUser == nil {
			http.Error(w, "Unauthorized - Invalid token", http.StatusUnauthorized)
			return
		}

		r.Header.Set("X-User-Email", foundUser.Email)
		next.ServeHTTP(w, r)
	}
}

func rateLimitMiddleware(next http.HandlerFunc) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if !limiter.Allow() {
			http.Error(w, "Too many requests", http.StatusTooManyRequests)
			return
		}
		next.ServeHTTP(w, r)
	}
}

func secureHeaders(next http.HandlerFunc) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("X-Content-Type-Options", "nosniff")
		w.Header().Set("X-Frame-Options", "DENY")
		w.Header().Set("X-XSS-Protection", "1; mode=block")
		w.Header().Set("Content-Security-Policy", "default-src 'self'")
		w.Header().Set("Strict-Transport-Security", "max-age=31536000; includeSubDomains")
		next.ServeHTTP(w, r)
	}
}

func validateEmail(email string) bool {
	return emailRegex.MatchString(email)
}

func handleRegister(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	var req RegisterRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, "Invalid request body", http.StatusBadRequest)
		return
	}

	// Enhanced validation
	if !validateEmail(req.Email) || len(req.Password) < 8 {
		http.Error(w, "Invalid email or password (password must be at least 8 characters)", http.StatusBadRequest)
		return
	}

	// Check if user exists
	if _, err := loadUser(req.Email); err == nil {
		http.Error(w, "User already exists", http.StatusConflict)
		return
	}

	// Hash password
	hashedPassword, err := bcrypt.GenerateFromPassword([]byte(req.Password), bcrypt.DefaultCost)
	if err != nil {
		http.Error(w, "Internal server error", http.StatusInternalServerError)
		return
	}

	// Generate token
	token, err := generateToken()
	if err != nil {
		http.Error(w, "Internal server error", http.StatusInternalServerError)
		return
	}

	// Create user
	user := &User{
		Email:     req.Email,
		Password:  string(hashedPassword),
		Token:     token,
		CreatedAt: time.Now(),
	}

	// Save user
	if err := saveUser(user); err != nil {
		http.Error(w, "Failed to save user", http.StatusInternalServerError)
		return
	}

	// Create user data directory
	userDataDir := getUserDataDir(req.Email)
	if err := os.MkdirAll(userDataDir, 0755); err != nil {
		http.Error(w, "Failed to create user directory", http.StatusInternalServerError)
		return
	}

	// Return user data (without password)
	user.Password = ""
	json.NewEncoder(w).Encode(user)
}

func handleLogin(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	var req LoginRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, "Invalid request body", http.StatusBadRequest)
		return
	}

	user, err := loadUser(req.Email)
	if err != nil {
		http.Error(w, "Invalid credentials", http.StatusUnauthorized)
		return
	}

	if err := bcrypt.CompareHashAndPassword([]byte(user.Password), []byte(req.Password)); err != nil {
		http.Error(w, "Invalid credentials", http.StatusUnauthorized)
		return
	}

	// Generate new token
	token, err := generateToken()
	if err != nil {
		http.Error(w, "Internal server error", http.StatusInternalServerError)
		return
	}

	user.Token = token
	if err := saveUser(user); err != nil {
		http.Error(w, "Failed to update user", http.StatusInternalServerError)
		return
	}

	// Return user data (without password)
	user.Password = ""
	json.NewEncoder(w).Encode(user)
}

func handleSync(w http.ResponseWriter, r *http.Request) {
	userEmail := r.Header.Get("X-User-Email")
	if userEmail == "" && r.Header.Get("X-User-Role") != "admin" {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}

	userDataDir := getUserDataDir(userEmail)
	syncFilePath := filepath.Join(userDataDir, "sync_data.json")

	switch r.Method {
	case http.MethodGet:
		data, err := os.ReadFile(syncFilePath)
		if err != nil {
			if os.IsNotExist(err) {
				json.NewEncoder(w).Encode(SyncData{
					Files:    make(map[string]string),
					Packages: make([]Package, 0),
				})
				return
			}
			http.Error(w, "Failed to read sync data", http.StatusInternalServerError)
			return
		}
		w.Header().Set("Content-Type", "application/json")
		w.Write(data)

	case http.MethodPost:
		var syncData SyncData
		if err := json.NewDecoder(r.Body).Decode(&syncData); err != nil {
			http.Error(w, "Invalid request body", http.StatusBadRequest)
			return
		}

		if err := os.MkdirAll(userDataDir, 0755); err != nil {
			http.Error(w, "Failed to create user directory", http.StatusInternalServerError)
			return
		}

		data, err := json.MarshalIndent(syncData, "", "  ")
		if err != nil {
			http.Error(w, "Failed to marshal sync data", http.StatusInternalServerError)
			return
		}

		if err := os.WriteFile(syncFilePath, data, 0644); err != nil {
			http.Error(w, "Failed to save sync data", http.StatusInternalServerError)
			return
		}

		w.WriteHeader(http.StatusOK)
		w.Write([]byte(`{"status": "ok"}`))

	default:
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
	}
}

func main() {
	// Load .env file
	if err := godotenv.Load(); err != nil {
		log.Println("Warning: .env file not found")
	}

	// Ensure directories exist with proper permissions
	for _, dir := range []string{dataDir, usersDir} {
		if err := os.MkdirAll(dir, 0755); err != nil {
			log.Fatal("Failed to create directory:", err)
		}
		// Ensure permissions are set correctly
		if err := os.Chmod(dir, 0755); err != nil {
			log.Fatal("Failed to set directory permissions:", err)
		}
	}

	// Check if admin token is set
	if os.Getenv("KIWI_AUTH_TOKEN") == "" {
		log.Fatal("KIWI_AUTH_TOKEN environment variable must be set")
	}

	// Create a new ServeMux for better route handling
	mux := http.NewServeMux()

	mux.HandleFunc("/health", func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(map[string]string{"status": "OK"})
	})

	// Apply middleware chain
	mux.HandleFunc("/register", secureHeaders(rateLimitMiddleware(handleRegister)))
	mux.HandleFunc("/login", secureHeaders(rateLimitMiddleware(handleLogin)))
	mux.HandleFunc("/sync", secureHeaders(rateLimitMiddleware(authMiddleware(handleSync))))

	port := os.Getenv("PORT")
	if port == "" {
		port = "8080"
	}

	server := &http.Server{
		Addr:         ":" + port,
		Handler:      mux,
		ReadTimeout:  15 * time.Second,
		WriteTimeout: 15 * time.Second,
		IdleTimeout:  60 * time.Second,
	}

	// Graceful shutdown setup
	done := make(chan bool)
	quit := make(chan os.Signal, 1)
	signal.Notify(quit, syscall.SIGINT, syscall.SIGTERM)

	go func() {
		<-quit
		log.Println("Server is shutting down...")

		ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
		defer cancel()

		server.SetKeepAlivesEnabled(false)
		if err := server.Shutdown(ctx); err != nil {
			log.Fatalf("Could not gracefully shutdown the server: %v\n", err)
		}
		close(done)
	}()

	log.Printf("Starting server on port %s", port)
	if err := server.ListenAndServe(); err != nil && err != http.ErrServerClosed {
		log.Fatal(err)
	}

	<-done
	log.Println("Server stopped")
}
