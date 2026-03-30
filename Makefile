# sublate Makefile

# Variables
TARGET_WASM = wasm32-unknown-unknown
WORKER_CRATE = packages/worker
WEB_CRATE = packages/web
WORKER_OUT = worker_out
ASSETS_DIR = packages/web/assets
DX_PUBLIC_DIR = target/dx/web/debug/web/public
DX_RELEASE_DIR = target/dx/web/release/web/public

# Polyfill script to be injected
POLYFILL = if(!console.createTask) console.createTask = function(n) { return { run: function(f) { return f(); } }; };

.PHONY: all help build-worker serve build-web build-release test clean patch-debug patch-release build setup check

help:
	@echo "sublate Makefile targets:"
	@echo "  serve          - Build worker and start Dioxus dev server"
	@echo "  build          - Build worker and web application (debug mode) with patches"
	@echo "  build-worker   - Build the multi-threaded worker and copy assets"
	@echo "  build-web      - Build the web application (debug)"
	@echo "  build-release  - Build the web application (release) and apply polyfills"
	@echo "  setup          - Install Node.js dependencies and Playwright browsers"
	@echo "  check          - Run cargo check on all packages"
	@echo "  patch-debug    - Apply console.createTask polyfill to the current debug build"
	@echo "  test           - Run Playwright tests"
	@echo "  clean          - Remove build artifacts and temporary files"

all: help

build: build-web

setup:
	@echo "Installing Playwright and Node.js dependencies..."
	npm install
	npx playwright install

check:
	@echo "Running cargo check..."
	cargo check -p web -p worker --target $(TARGET_WASM)

build-worker:
	@echo "Building worker..."
	cargo build -p worker --target $(TARGET_WASM) --release
	wasm-bindgen target/$(TARGET_WASM)/release/worker.wasm --out-dir $(WORKER_OUT) --target web --no-typescript
	cp $(WORKER_OUT)/worker.js $(ASSETS_DIR)/worker_lib.js
	cp $(WORKER_OUT)/worker_bg.wasm $(ASSETS_DIR)/worker_bg.wasm
	@echo "Worker built and assets copied."

serve: build-worker
	@echo "Starting Dioxus dev server..."
	@echo "Note: If you see 'console.createTask' errors in the browser, run 'make patch-debug' once the server is running."
	dx serve --package web

build-web: build-worker
	@echo "Building web application (debug mode)..."
	dx build --package web
	@$(MAKE) patch-debug

build-release: build-worker
	@echo "Building web application (release mode)..."
	dx build --package web --release
	@$(MAKE) patch-release

patch-debug:
	@echo "Patching debug build with console.createTask polyfill..."
	@sed -i '1i$(POLYFILL)' $(DX_PUBLIC_DIR)/wasm/web.js
	@sed -i '/<head>/a <script>$(POLYFILL)</script>' $(DX_PUBLIC_DIR)/index.html
	@echo "Debug build patched."

patch-release:
	@echo "Patching release build with console.createTask polyfill..."
	@sed -i '1i$(POLYFILL)' $(DX_RELEASE_DIR)/wasm/web.js
	@sed -i '/<head>/a <script>$(POLYFILL)</script>' $(DX_RELEASE_DIR)/index.html
	@echo "Release build patched."

test:
	@echo "Running Playwright tests..."
	npx playwright test

clean:
	@echo "Cleaning up..."
	cargo clean
	rm -rf $(WORKER_OUT)
	rm -rf node_modules
	rm -rf target/dx
	rm -f $(ASSETS_DIR)/worker_lib.js $(ASSETS_DIR)/worker_bg.wasm
