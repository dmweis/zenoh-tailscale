TARGET_HOST ?= homepi
TARGET_USERNAME ?= $$USER
TARGET_HOST_USER ?= $(TARGET_USERNAME)@$(TARGET_HOST)

.PHONY: build
build:
	cargo build --release

.PHONY: build-deb
build-deb: build
	cargo deb --no-build --fast

.PHONE: install
install: build-deb
	sudo dpkg -i target/debian/zenoh-tailscale*.deb

.PHONY: install-dependencies
install-dependencies:
	sudo apt update && sudo apt install libudev-dev liblzma-dev libclang-dev -y
	cargo install cargo-deb

.PHONY: build-docker
build-docker:
	rm -rf docker_out
	mkdir docker_out
	DOCKER_BUILDKIT=1 docker build --tag zenoh-tailscale-builder --file Dockerfile --output type=local,dest=docker_out .

.PHONY: push-docker
push-docker: build-docker
	rsync -avz --delete docker_out/* $(TARGET_HOST_USER):/home/$(TARGET_USERNAME)/zenoh-tailscale/

.PHONY: deploy-docker
deploy-docker: push-docker
	@echo "Installing zenoh-tailscale on $(TARGET_HOST)"
	mosquitto_pub -h homepi -t "zenoh-tailscale/build" -n

.PHONY: upload-release-github
upload-release-github: build-docker build-deb
	gh release create v$$(cargo get package.version) --title v$$(cargo get package.version) --notes "" docker_out/zenoh-tailscale docker_out/zenoh-tailscale.deb docker_out/zenoh-tailscale_*.deb
	# TODO: Figure out how to upload non raspberry pi packages alongside the main ones
	# gh release upload $$(cargo get package.version) target/release/zenoh-tailscale target/debian/zenoh-tailscale_*.deb
	@echo deb image at https://github.com/dmweis/zenoh-tailscale/releases/latest/download/zenoh-tailscale.deb

.PHONY: upload-release-github-amd64
upload-release-github-amd64: build-deb
	gh release upload v$$(cargo get package.version) target/debian/zenoh-tailscale_*amd64.deb
	@echo deb image at https://github.com/dmweis/zenoh-tailscale/releases/latest/download/zenoh-tailscale.deb
