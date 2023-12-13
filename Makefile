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
	rsync -avz --delete docker_out/* $(TARGET_HOST_USER):/home/$(TARGET_USERNAME)/zenoh-tailscale

.PHONY: deploy-docker
deploy-docker: push-docker
	@echo "Installing zenoh-tailscale on $(TARGET_HOST)"
	mosquitto_pub -h homepi -t "zenoh-tailscale/build" -n
