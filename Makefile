CONTAINER_NAME ?= rcore-container-qemu8
DOCKER_NAME ?= rcore-docker-qemu8
.PHONY: docker build_docker attach_docker rebuild_docker fmt

docker:
	if ! docker images --format "{{.Repository}}" | grep -q "^${DOCKER_NAME}$$"; then \
		echo "❌ image '${DOCKER_NAME}' not exits，start building..."; \
		make build_docker; \
	else \
		echo "✅ image '${DOCKER_NAME}' was built, skip building..."; \
	fi; \
	if docker ps -a --filter "name=^/${CONTAINER_NAME}$$" --format "{{.Names}}" | grep -q "${CONTAINER_NAME}"; then \
		echo "✅ find existing container: ${CONTAINER_NAME}，accessing..."; \
		if ! docker ps --filter "name=^/${CONTAINER_NAME}$$" --format "{{.Names}}" | grep -q "${CONTAINER_NAME}"; then \
			echo "🔧 container isn't running，start launching..."; \
			docker start ${CONTAINER_NAME}; \
		fi; \
		docker exec -it ${CONTAINER_NAME} bash; \
	else \
		echo "🚀 no existing container is found: ${CONTAINER_NAME}，start creating new container..."; \
		docker run --network host -it -d \
			--name ${CONTAINER_NAME} \
			-v ${CURDIR}:/mnt \
			-w /mnt \
			${DOCKER_NAME} \
			bash; \
		docker exec -it ${CONTAINER_NAME} bash; \
	fi

# build docker if container doesn't exist
build_docker:
	docker build -t ${DOCKER_NAME} .

fmt:
	cd easy-fs; cargo fmt; cd ../easy-fs-fuse cargo fmt; cd ../os ; cargo fmt; cd ../user; cargo fmt; cd ..

attach_docker:
	@if docker ps -a --filter "name=^/${CONTAINER_NAME}$$" --format "{{.Names}}" | grep -q "${CONTAINER_NAME}"; then \
		if ! docker ps --filter "name=^/${CONTAINER_NAME}$$" --format "{{.Names}}" | grep -q "${CONTAINER_NAME}"; then \
			docker start ${CONTAINER_NAME}; \
		fi; \
		docker exec -it ${CONTAINER_NAME} bash; \
	else \
		echo "❌ no existing container is found: ${CONTAINER_NAME}，please run 'make docker' to create container"; \
	fi

rebuild_docker:
	@echo "🗑️ delete existing container ${CONTAINER_NAME}..."; \
	docker rm -f ${CONTAINER_NAME} 2>/dev/null; \
	echo "🚀 start creating container..."; \
	docker run --network host -it -d \
		--name ${CONTAINER_NAME} \
		-v ${CURDIR}:/mnt \
		-w /mnt \
		${DOCKER_NAME} \
		bash; \
	docker exec -it ${CONTAINER_NAME} bash;

fmt:
	cd os ; cargo fmt; cd ..
