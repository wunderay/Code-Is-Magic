// Go receiver syntax and flowing triad, emitted with the standard JSON encoder.
package main

import (
	"encoding/json"
	"os"
)

type Path struct {
	Op   string `json:"op"`
	Data string `json:"data"`
}
type Profile struct {
	ID         string   `json:"id"`
	Base       string   `json:"base"`
	Name       string   `json:"name"`
	Extensions []string `json:"extensions"`
	CallGroup  string   `json:"call_group"`
	Receiver   string   `json:"receiver_keyword"`
	Sigil      string   `json:"sigil"`
	Repeats    int      `json:"repeats"`
	Collar     []Path   `json:"collar"`
}

func main() {
	profile := Profile{"go", "c", "Go", []string{"go"}, "go", "func", "≋", 3,
		[]Path{{"path", "M -.12 -.965 Q -.04 -.99 0 -.955 T .12 -.965 M -.1 -.938 Q -.03 -.965 .02 -.936 T .1 -.938"}}}
	if err := json.NewEncoder(os.Stdout).Encode(profile); err != nil {
		panic(err)
	}
}
