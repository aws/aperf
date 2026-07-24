# Running APerf on EKS/Kubernetes

This guide explains how to run APerf on Amazon EKS (Elastic Kubernetes Service) or any Kubernetes cluster to collect performance metrics without requiring SSH access to the Kubernetes nodes.

## Overview

This repository provides an automated script (`eks-aperf.sh`) that deploys APerf profiling pods on Amazon EKS worker nodes. It creates a privileged pod on a target node, runs performance profiling, generates reports, and copies the results locally.

## Features

- **Node-specific deployment**: Target specific EKS worker nodes for profiling
- **Automated profiling**: Runs APerf record and report generation automatically
- **Security validation**: Checks namespace security policies before deployment
- **Resource monitoring**: Shows current resource usage on target nodes
- **Automatic cleanup**: Removes pods after profiling completion
- **Flexible configuration**: Customizable CPU/memory limits and APerf options
- **Multi-architecture support**: Works with both AMD64 and ARM64 nodes

## Prerequisites

Before you begin, ensure you have the following tools installed and configured on your laptop:

- **Git** - To clone this repository
- **kubectl** - Installed and connected to your EKS cluster
- **Docker** - To build the APerf container image
- **AWS CLI** - With AWS credentials configured for ECR access
- **jq** - For JSON parsing (used in scripts)
- Appropriate RBAC permissions to create pods in the target namespace
- Target namespace must allow privileged pods

### Worker Node Requirements

- **Host path** `/opt/k8s/async-profiler/async-profiler-4.2-linux-arm64` must exist
- **Async Profiler binaries** must be present in the host path directory
- Directory should contain the complete async-profiler installation
- Proper file permissions for profiler execution

### Application Pod Requirements

For the APerf pod to successfully profile your application pods, **both the application pod and APerf pod must mount the same async profiler host path**:

**Application Pod Configuration:**
```yaml
volumeMounts:
- name: profiler-vol
  mountPath: /opt/async-profiler
volumes:
- name: profiler-vol
  hostPath:
    path: /opt/k8s/async-profiler/async-profiler-4.2-linux-arm64
    type: DirectoryOrCreate
```

**Important**: Both pods must use the **exact same host path** to ensure the profiler can access and profile the target application processes.

## Setup Instructions

### Step 1: Clone Repository

Clone this repository to your laptop:
```bash
git clone https://github.com/jrishabh248/aperf 
cd aperf
```

### Step 2: Verify Prerequisites

Ensure kubectl is connected to your EKS cluster:
```bash
kubectl get nodes
```

Verify AWS credentials are configured:
```bash
aws sts get-caller-identity
```

### Step 3: Create ECR Repository

Create an Amazon ECR repository to contain the APerf image:

```bash
# Optional: Set your AWS region if needed
# export AWS_REGION="us-west-2"

# Optional: Set AWS profile if needed
# export AWS_PROFILE="your-profile-name"

# Create ECR repository
aws ecr create-repository --repository-name aperf --region $AWS_REGION

# Get ECR repository URL
APERF_ECRREPO=$(aws ecr describe-repositories --repository-names aperf --region $AWS_REGION | jq -r '.repositories[0].repositoryUri')
echo "ECR Repository URL: $APERF_ECRREPO"

# Authenticate with ECR
aws ecr get-login-password --region $AWS_REGION | docker login --username AWS --password-stdin $APERF_ECRREPO
```

### Step 4: Build and Push APerf Container Image

Build the multi-architecture APerf container image and push it to ECR:

```bash
# Build and push multi-architecture image (supports both AMD64 and ARM64)
docker buildx build --push --platform linux/amd64,linux/arm64 -t ${APERF_ECRREPO}:latest -f ./Dockerfile .
```

**Note**: If you don't have `docker buildx` configured for multi-platform builds, you can build for your specific architecture:

```bash
docker build -t ${APERF_ECRREPO}:latest -f ./Dockerfile .
docker push ${APERF_ECRREPO}:latest
```

## Usage

### Basic Usage

```bash
./eks-aperf.sh --aperf_image=<ECR_IMAGE_URL> --node=<NODE_NAME>
```

### Advanced Usage

```bash
./eks-aperf.sh \
  --aperf_image=900063036704.dkr.ecr.ap-south-1.amazonaws.com/aperf \
  --node=ip-192-168-81-49.ap-south-1.compute.internal \
  --namespace=profiling \
  --aperf_options="--profile --profile-java -i 1 -p 150" \
  --cpu-request=2.0 \
  --memory-request=2Gi \
  --cpu-limit=8.0 \
  --memory-limit=8Gi
```

### Identify Target Node

Find a target Kubernetes node where you want to collect APerf metrics:

```bash
# List all pods with their nodes to identify a target node
kubectl get pods -A -o wide
```

Example node name: `ip-10-0-120-104.us-west-2.compute.internal`

## Parameters

| Parameter | Required | Default | Description |
|-----------|----------|---------|-------------|
| `--aperf_image` | Yes | - | ECR image URL containing APerf profiler |
| `--node` | Yes | - | Kubernetes node name to run profiling on |
| `--namespace` | No | `default` | Kubernetes namespace for pod deployment |
| `--aperf_options` | No | `""` | Additional options to pass to APerf |
| `--cpu-request` | No | `1.0` | CPU request for the profiling pod |
| `--memory-request` | No | `1Gi` | Memory request for the profiling pod |
| `--cpu-limit` | No | `4.0` | CPU limit for the profiling pod |
| `--memory-limit` | No | `4Gi` | Memory limit for the profiling pod |
| `--help` | No | - | Show help message |

## What the Script Does

1. **Validates inputs**: Checks required parameters and cluster access
2. **Node validation**: Verifies target node exists and shows instance type
3. **Security check**: Validates namespace security policies for privileged pods
4. **Resource monitoring**: Displays current resource usage on target node
5. **Pod deployment**: Creates and deploys APerf profiling pod on target node
6. **Profiling execution**: Runs APerf record with specified options
7. **Report generation**: Creates APerf report and packages it as tar.gz
8. **File retrieval**: Copies profiling results to local directory
9. **Cleanup**: Removes the profiling pod from the cluster

## Pod Configuration

The script creates a privileged pod with the following characteristics:

- **Privileged security context**: Required for system-level profiling
- **Host PID namespace**: Access to host processes
- **Host network**: Direct network access
- **Volume mounts**:
  - `/boot` (read-only): Access to kernel symbols
  - `/opt/async-profiler`: APerf profiler binaries
- **Node selector**: Ensures pod runs on specified node

## Example Commands

### Profile for 60 seconds with profiling enabled
```bash
bash ./eks-aperf.sh \
  --aperf_image="${APERF_ECRREPO}:latest" \
  --node="ip-10-0-120-104.us-west-2.compute.internal" \
  --aperf_options="-p 60 --profile" \
  --namespace="aperf"
```

### Profile with custom resource limits
```bash
bash ./eks-aperf.sh \
  --aperf_image="${APERF_ECRREPO}:latest" \
  --node="ip-10-0-120-104.us-west-2.compute.internal" \
  --cpu-request="2.0" \
  --memory-request="2Gi" \
  --cpu-limit="8.0" \
  --memory-limit="8Gi"
```

### Profile Java applications with 1-second intervals
```bash
./eks-aperf.sh \
  --aperf_image=your-ecr-repo/aperf:latest \
  --node=ip-10-0-1-100.ec2.internal \
  --aperf_options="--profile-java -i 1 -d 60"
```

## Output Files

The script generates timestamped files:
- `aperf_report_YYYYMMDD-HHMMSS.tar.gz`: Complete profiling report archive

## Example Output

```bash
$ bash ./eks-aperf.sh --aperf_image="${APERF_ECRREPO}:latest" --namespace=aperf --node ip-10-0-120-104.us-west-2.compute.internal --aperf_options="-p 30 --profile"

Tageted node instance type...   m6g.8xlarge
Check namespace security policy...   Namespace 'aperf' has 'privileged' policy - privileged pods allowed.
Resource usage for pods on ip-10-0-120-104.us-west-2.compute.internal:
NAMESPACE     NAME                                            CPU(cores)   MEMORY(bytes)
kube-system   aws-node-fddbt                                  3m           58Mi
kube-system   ebs-csi-node-dgjl9                              1m           31Mi
kube-system   kube-proxy-ct4n5                                1m           15Mi
mongodb       mongodb-5bd6669d6b-kmn4w                        723m         1636Mi

Created pod configuration for node: ip-10-0-120-104.us-west-2.compute.internal
Deploying pod to Kubernetes...  pod/aperf-pod-ip-10-0-120-104-us-west-2-compute-internal created
Waiting for pod to start...  pod/aperf-pod-ip-10-0-120-104-us-west-2-compute-internal condition met
Running aperf profiling...
Generating aperf report...
Copying files from pod aperf-pod-ip-10-0-120-104-us-west-2-compute-internal...
Deleting pod to clean up resources...  pod "aperf-pod-ip-10-0-120-104-us-west-2-compute-internal" deleted
Files copied to aperf_report_20250626-133204.tar.gz
Done!
```

## Troubleshooting

### Common Issues

1. **"Privileged pods NOT allowed"**
   - The namespace has security restrictions
   - Use a namespace that allows privileged pods or modify security policies

2. **"Pod failed to reach ready state"**
   - Check node resources and availability
   - Verify ECR image accessibility from the node
   - Review pod logs for specific errors

3. **"No aperf report directory found"**
   - APerf profiling may have failed
   - Check APerf options and target processes
   - Verify profiler has necessary permissions

### Getting Node Names
```bash
kubectl get nodes
```

### Checking Namespace Security Policies
```bash
kubectl get namespace <namespace> -o yaml | grep -A5 labels
```

## Security Considerations

- The APerf pod runs with privileged access to collect all system-level metrics
- The pod has access in read-only mode to the host's `/boot` directory and processes PIDs
- Ensure your cluster's security policies allow privileged pods if required
- The pod is automatically cleaned up after execution
- Only use on trusted clusters and nodes
- Ensure ECR images are from trusted sources
- Monitor resource usage during profiling



## Requirements Summary

- Kubernetes cluster with worker nodes
- kubectl access with pod creation permissions
- ECR access for pulling profiler images
- Sufficient node resources for profiling overhead
- **Worker Node Setup**:
  - Host directory `/opt/k8s/async-profiler/async-profiler-4.2-linux-arm64` must exist
  - Async Profiler binaries installed in the host path
  - Proper file permissions for profiler execution
- **Application Pod Configuration**:
  - Application pods must mount the same async profiler host path
  - Both application and APerf pods need identical volume mount configuration
  - Shared host path enables profiler access to target processes
