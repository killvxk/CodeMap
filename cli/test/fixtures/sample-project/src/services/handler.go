package services

import (
	"fmt"
	"net/http"
	"encoding/json"
)

// Handler handles HTTP requests.
type Handler struct {
	db *Database
}

// Response represents an API response.
type Response struct {
	Code    int    `json:"code"`
	Message string `json:"message"`
}

// Servicer defines the service interface.
type Servicer interface {
	Process(input string) (string, error)
}

// NewHandler creates a new Handler.
func NewHandler(db *Database) *Handler {
	return &Handler{db: db}
}

// ServeHTTP handles the request.
func (h *Handler) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	resp := Response{Code: 200, Message: "ok"}
	json.NewEncoder(w).Encode(resp)
}

func internalHelper() {
	fmt.Println("not exported")
}
