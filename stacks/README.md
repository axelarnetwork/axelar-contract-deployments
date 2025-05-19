# Stacks Deployment Guide

## Table of Contents

-   [Prerequisites](#prerequisites)
-   [Deployment](#deployment)
-   [Contract Upgrades](#contract-upgrades)
-   [Contract Interactions](#contract-interactions)
-   [Examples](#examples)
-   [Troubleshooting](#troubleshooting)

## Prerequisites

A Stacks contracts deployer account can be created as follows.

```bash
node stacks/generate-wallet.js
```

Set `STACKS_MNEMONIC="stacksmnemonic..."` in your `.env` file.

## Deployment

To get test Stacks coins to your address via a faucet.

```bash
node stacks/faucet.js
```
