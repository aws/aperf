# Running APerf on EKS/Kubernetes

This guide explains how to run APerf on Amazon EKS (Elastic Kubernetes Service) or any Kubernetes cluster to collect performance metrics without requiring SSH access to the Kubernetes nodes.

## Prerequisites

Before you begin, ensure you have the following tools installed and configured on your laptop:

- **Git** - To clone this repository
- **kubectl** - Installed and connected to your EKS cluster
- **Docker** - To build the APerf container image
- **AWS CLI** - With AWS credentials configured for ECR access
- **jq** - For JSON parsing (used in scripts)

## Requirements

1. Clone this repository to your laptop:
   ```bash
   git clone https://github.com/aws/aperf 
   cd aperf
   ```

2. Ensure kubectl is connected to your EKS cluster:
   ```bash
   kubectl get nodes
   ```

3. Verify AWS credentials are configured:
   ```bash
   aws sts get-caller-identity
   ```

## Setup Instructions

### Step 1: Create ECR Repository

First, create an Amazon ECR repository to contain the APerf image:

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

### Step 2: Build and Push APerf Container Image

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

Your APerf containerized image should now be available on the ECR registry.


### Step 3: Run APerf on EKS

#### 3a. Identify Target Node

Find a target Kubernetes node where you want to collect APerf metrics:

```bash
# List all pods with their nodes to identify a target node
kubectl get pods -A -o wide
```

Example node name: `ip-10-0-120-104.us-west-2.compute.internal` or `i-02a3f32795d5d95c2`

#### 3b. Execute APerf Collection

Use the provided `eks-aperf.sh` script to run APerf on the selected node:

```bash
bash ./eks-aperf.sh \
  --aperf_image="${APERF_ECRREPO}:latest" \
  --node="ip-10-0-120-104.us-west-2.compute.internal" 
```

##### Script Parameters

- `--aperf_image`: ECR image URL for the APerf container
- `--node`: Target Kubernetes node name
- `--aperf_options`: APerf command options (optional, default: ``)
- `--namespace`: Kubernetes namespace (optional, default: `default`)
- `--cpu-request`: CPU request for the pod (optional, default: `1.0`)
- `--memory-request`: Memory request for the pod (optional, default: `1Gi`)
- `--cpu-limit`: CPU limit for the pod (optional, default: `4.0`)
- `--memory-limit`: Memory limit for the pod (optional, default: `4Gi`)

##### Example with Custom Options

```bash
# Run APerf for 60 seconds with profiling enabled
bash ./eks-aperf.sh \
  --aperf_image="${APERF_ECRREPO}:latest" \
  --node="ip-10-0-120-104.us-west-2.compute.internal" \
  --aperf_options="-p 60 --profile" \
  --namespace="aperf"
```

#### Example with Custom Resource Limits

```bash
# Run APerf with custom CPU and memory settings
bash ./eks-aperf.sh \
  --aperf_image="${APERF_ECRREPO}:latest" \
  --node="ip-10-0-120-104.us-west-2.compute.internal" \
  --cpu-request="2.0" \
  --memory-request="2Gi" \
  --cpu-limit="8.0" \
  --memory-limit="8Gi"
```

#### 3c. Collect Results

The `eks-aperf.sh` script will automatically run the following steps:

1. **Pod Deployment**: Deploy a privileged pod on the specified node
2. **APerf Record**: Runs APerf record inside the pod with the specified options
3. **APerf Report**: Runs APerf report generation inside the pod
4. **File Transfer**: Copies the generated report from the pod to your local machine
5. **Cleanup**: Removes the pod after successful completion

The APerf report will be downloaded as a compressed tarball file with a timestamp (E.g. aperf_report_20250626-133204.tar.gz)

Example of correct output execution of the script:
```bash
$ bash ./eks-aperf.sh --aperf_image="${APERF_ECRREPO}:latest"  --namespace=aperf --node  ip-10-0-120-104.us-west-2.compute.internal  --aperf_options="-p 30 --profile"

Tageted node instance type...   m6g.8xlarge
Check namespace security policy...   Namespace 'aperf' has 'privileged' policy - privileged pods allowed.
Resource usage for pods on ip-10-0-120-104.us-west-2.compute.internal:
NAMESPACE     NAME                                            CPU(cores)   MEMORY(bytes)
kube-system   aws-node-fddbt                                  3m           58Mi
kube-system   ebs-csi-node-dgjl9                              1m           31Mi
kube-system   kube-proxy-ct4n5                                1m           15Mi
mongodb       mongodb-5bd6669d6b-kmn4w                        723m         1636Mi
mongodb       mongodb-ycsb                                    303m         616Mi
postgresql    cassandra-db8f77cc8-mq6kh                       2545m        34613Mi
postgresql    cassandra-server                                1057m        1033Mi
postgresql    pg-deployment-7d775cdcdd-s76bg                  15735m       17085Mi
postgresql    pg-postgresql-client                            2866m        2Mi

Created pod configuration for node: ip-10-0-120-104.us-west-2.compute.internal
Deploying pod to Kubernetes...  pod/aperf-pod-ip-10-0-120-104-us-west-2-compute-internal created
Waiting for pod to start...  pod/aperf-pod-ip-10-0-120-104-us-west-2-compute-internal condition met
Starting program logs...

Starting Aperf recording execution...
Run: /usr/bin/aperf record -r aperf_record -p 30 --profile
[2025-06-26T20:30:08Z INFO  aperf::record] Starting Data collection...
[2025-06-26T20:30:08Z INFO  aperf::record] Preparing data collectors...
[2025-06-26T20:30:49Z INFO  aperf::record] Collecting data...
[2025-06-26T20:31:37Z INFO  aperf::data::flamegraphs] Creating flamegraph...
[2025-06-26T20:31:50Z INFO  aperf::record] Data collection complete.
[2025-06-26T20:31:54Z INFO  aperf] Data collected in aperf_record/, archived in aperf_record.tar.gz
APerf record completed

Starting Aperf report generation...
Run: /usr/bin/aperf report -r aperf_record -n aperf_report
[2025-06-26T20:31:54Z INFO  aperf::report] Creating APerf report...
[2025-06-26T20:32:01Z INFO  aperf::report] Generating aperf_report.tar.gz
APerf report generation completed

Waiting for files to be copied...
Aperf completed. Copying files from pod aperf-pod-ip-10-0-120-104-us-west-2-compute-internal...
Deleting pod to clean up resources...  pod "aperf-pod-ip-10-0-120-104-us-west-2-compute-internal" deleted
Files copied to aperf_report_20250626-133204.tar.gz
Done!
```


## Security Considerations

- The APerf pod runs with privileged access to collect all system-level metrics
- The pod has access in read-only mode to the host's `/boot` directory and processes PIDs
- Ensure your cluster's security policies allow privileged pods if required
- The pod is automatically cleaned up after execution


## Known Limitations

**Note**: The `--profile-java` option is not currently fully supported with this script.
