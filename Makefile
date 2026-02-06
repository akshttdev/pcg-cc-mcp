.PHONY: help setup deploy start stop restart status logs logs-app logs-nginx logs-apn logs-cloudflared backup update clean shell check

# Default target
.DEFAULT_GOAL := help

help: ## Show this help message
	@echo ""
	@echo "🚀 PCG-CC-MCP Docker Deployment"
	@echo "================================"
	@echo ""
	@echo "Available commands:"
	@echo ""
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2}'
	@echo ""
	@echo "Quick Start:"
	@echo "  make setup    # Configure environment"
	@echo "  make deploy   # Build and start services"
	@echo ""

setup: ## Setup and configure environment
	@./deploy.sh setup

deploy: ## Build and deploy (fresh start)
	@./deploy.sh deploy

start: ## Start existing containers
	@./deploy.sh start

stop: ## Stop all containers
	@./deploy.sh stop

restart: ## Restart all containers
	@./deploy.sh restart

status: ## Show service status
	@./deploy.sh status

logs: ## Show logs for all services
	@docker-compose logs -f

logs-app: ## Show logs for main application
	@docker-compose logs -f app

logs-nginx: ## Show logs for nginx
	@docker-compose logs -f nginx

logs-apn: ## Show logs for APN bridge
	@docker-compose logs -f apn-bridge

logs-cloudflared: ## Show logs for Cloudflare tunnel
	@docker-compose logs -f cloudflared

backup: ## Create manual database backup
	@./deploy.sh backup

update: ## Update and rebuild application
	@./deploy.sh update

clean: ## Clean rebuild from scratch
	@./deploy.sh clean

shell: ## Open shell in app container
	@docker-compose exec app /bin/bash

check: ## Run system checks
	@./deploy.sh check

# Monitoring
ps: ## Show running containers
	@docker-compose ps

stats: ## Show resource usage
	@docker stats --format "table {{.Container}}\t{{.CPUPerc}}\t{{.MemUsage}}\t{{.NetIO}}\t{{.BlockIO}}"

# Database operations
db-shell: ## Open SQLite shell
	@docker-compose exec app sqlite3 /app/dev_assets/db.sqlite

db-size: ## Show database size
	@docker-compose exec app du -h /app/dev_assets/db.sqlite

# Development
dev-logs: ## Show development logs with timestamps
	@docker-compose logs -f --timestamps

tail: ## Tail logs (last 100 lines)
	@docker-compose logs --tail=100 -f

# Cleanup
prune: ## Remove unused Docker resources
	@docker system prune -f
	@docker volume prune -f

deep-clean: ## Stop and remove everything
	@docker-compose down -v --remove-orphans
	@docker system prune -af

# Health checks
health: ## Check service health
	@echo "🏥 Checking service health..."
	@echo ""
	@echo "App Service:"
	@curl -sf http://localhost:3001/api/health || echo "❌ App not responding"
	@echo ""
	@echo "Ollama Service:"
	@curl -sf http://localhost:11434/api/tags > /dev/null && echo "✅ Ollama OK" || echo "❌ Ollama not responding"
	@echo ""
	@echo "Chatterbox Service:"
	@curl -sf http://localhost:8100 > /dev/null && echo "✅ Chatterbox OK" || echo "❌ Chatterbox not responding"
	@echo ""
	@echo "Nginx Service:"
	@curl -sf http://localhost:8080 > /dev/null && echo "✅ Nginx OK" || echo "❌ Nginx not responding"
