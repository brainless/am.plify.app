# am.plify.app

A network where founders and business owners help amplify each other's products/services through genuine social media engagement and reviews.

## Project Overview

**Core Concept**: Founders help promote each other's products on social media platforms (Reddit, Twitter, LinkedIn) in exchange for points. All engagement must be genuine - users must actually try the product/service before helping.

**Key Features**:
- User registration with social login support (Google, Apple, Twitter, LinkedIn, GitHub)
- Product/service listings with trial access management
- Help submission system with screenshot proof
- Points-based reward system (users start with 99 points, exchange points when helping)
- Direct messaging for trial access requests
- Daily limits (max 3 activities per day) to ensure quality

## Tech Stack

**Backend**: Rust + Actix Web + SQLite + Redis + LiteStream backups
**Frontend**: TypeScript + SolidJS + Tailwind CSS + Vite
**Hosting**: Scaleway (backend at api.plify.app) + CloudFlare Pages (frontend at am.plify.app)
**Email**: Resend for transactional emails
**Infrastructure**: Terraform + Ansible for server management

## Key Technical Details

- **Configuration-driven**: All help activities, business domains, form fields defined in YAML specs
- **TypeScript Generation**: Backend generates TS types from YAML for frontend consumption
- **Caching**: Redis for all database reads, write-through cache invalidation
- **Testing Priority**: Integration and E2E tests over unit tests
- **MVP Focus**: Mobile-first design, basic features only, no admin dashboard

## Development Workflow
- Create a new branch for each task
- Branch names should start with chore/ or feature/ or fix/
- Please add tests for any new features added, particularly integration tests
- Please run formatters, linters and tests before committing changes
- When finished please commit and push to the new branch
- Please mention GitHub issue if provided
- After working on an issue from GitHub, update issue's tasks and open PR
