NAME := "valence-coprocessor-utils"
VERSION := "0.3.7"
HUB := "vtw11/valence"

all: build ## Build all targets.

build: ## Build the Docker image.
	docker build -t $(NAME):$(VERSION) .

tag: build ## Tag the built image.
	docker tag $(NAME):$(VERSION) $(HUB):$(VERSION)

publish: tag ## Publish the tagged image.
	docker push $(HUB):$(VERSION)

help: ## Display this help screen
	@echo -e "\033[1;37m$(PROJECT) ($(VERSION))\033[0m"
	@grep -h -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-15s\033[0m %s\n", $$1, $$2}'

.PHONY: help all build tag publish
