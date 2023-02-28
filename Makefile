IMAGE_NAME = tf-unused
GIT_TAG = $(shell git describe --tags HEAD)
TAG ?= $(GIT_TAG)
DOCKER_TAG ?= $(IMAGE_NAME):$(TAG)

require:
	@pip --version >/dev/null 2>&1 || (echo "ERROR: pip is required. Please install python/pip via pyenv:\n		https://github.com/pyenv/pyenv"; exit 1)

init: require
	@pre-commit --version >/dev/null 2>&1 || (pip install pre-commit)
	@pre-commit install >/dev/null 2>&1
	@echo "Init complete! Happy coding :)"

test-lint:
	@pre-commit run --all

test: test-lint

clean:
	rm -rf ./target

build: clean
	docker run --rm \
		-v `pwd`:/workdir -w /workdir \
		--network=host \
		-u $(shell id -u ${USER}):$(shell id -g ${USER}) \
		rust:1.67-bullseye cargo install --path . --target-dir ./target

publish:
	gh release upload \
		--repo mijdavis2/tf-unused \
		$(TAG) target/release/tf-unused

test-docker:
	docker build --network=host -t $(DOCKER_TAG) .
	docker run --rm $(DOCKER_TAG) tf-unused --version
