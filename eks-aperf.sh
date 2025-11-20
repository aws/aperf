#!/usr/bin/env bash

set -e
set -o pipefail

# Script to deploy a pod on a specific node, run aperf, and copy files locally
# Usage: ./aperf_k8s.sh --node="node-name" [--namespace="namespace"] [--aperf_options="options"] [--help]
#    or: ./aperf_k8s.sh --node "node-name" [--namespace "namespace"] [--aperf_options "options"] [--help]

# Default values
NAMESPACE="default"
APERF_OPTIONS=""
NODE_NAME=""
APERF_IMAGE=""
SHOW_HELP=false
CPU_REQUEST="1.0"
MEMORY_REQUEST="1Gi"
CPU_LIMIT="4.0"
MEMORY_LIMIT="4Gi"

# Define color and formatting codes
BOLD="\033[1m"
GREEN="\033[0;32m"
YELLOW="\033[0;33m"
BLUE="\033[0;34m"
RED="\033[0;31m"
NC="\033[0m" # No Color

# Parse command line arguments
while [ $# -gt 0 ]; do
  dest=""
  case "${1%=*}" in
    --node) dest="NODE_NAME";;
    --namespace) dest="NAMESPACE";;
    --aperf_options) dest="APERF_OPTIONS";;
    --aperf_image) dest="APERF_IMAGE";;
    --cpu-request) dest="CPU_REQUEST";;
    --memory-request) dest="MEMORY_REQUEST";;
    --cpu-limit) dest="CPU_LIMIT";;
    --memory-limit) dest="MEMORY_LIMIT";;
    --help)
      SHOW_HELP=true
      shift
      continue
      ;;
    *)
      echo "Unknown parameter: $1"
      exit 1
      ;;
  esac
  
  if [[ "$1" = *=* ]]; then
    eval ${dest}='"${1#*=}"'
  else
    eval ${dest}='"$2"'
    shift
  fi
  shift
done

# Show help if requested
if [ "$SHOW_HELP" = true ]; then
  echo "Usage: ./aperf_k8s.sh --aperf_image=IMAGE --node=NODE_NAME [--namespace=NAMESPACE] [--aperf_options=OPTIONS] [--help]"
  echo "   or: ./aperf_k8s.sh --aperf_image IMAGE --node NODE_NAME [--namespace NAMESPACE] [--aperf_options OPTIONS] [--help]"
  echo ""
  echo "Parameters:"
  echo "  --aperf_image    Required. ECR image location"
  echo "  --node           Required. The name of the Kubernetes node to run aperf on"
  echo "  --namespace      Optional. The Kubernetes namespace (default: '${NAMESPACE}')"
  echo "  --aperf_options  Optional. Options to pass to aperf (default: '${APERF_OPTIONS}')"
  echo "  --cpu-request    Optional. CPU request (default: '${CPU_REQUEST}')"
  echo "  --memory-request Optional. Memory request (default: '${MEMORY_REQUEST}')"
  echo "  --cpu-limit      Optional. CPU limit (default: '${CPU_LIMIT}')"
  echo "  --memory-limit   Optional. Memory limit (default: '${MEMORY_LIMIT}')"
  echo "  --help           Show this help message"
  exit 0
fi

# Check if aperf image is provided
if [ -z "$APERF_IMAGE" ]; then
  echo "Error: APerf image is required. Use --aperf_image=IMAGE_URL or --aperf_image IMAGE_URL to specify the ECR image location."
  echo "Use --help for more information."
  exit 1
fi

# Check if node name is provided
if [ -z "$NODE_NAME" ]; then
  echo "Error: Node name is required. Use --node=NODE_NAME or --node NODE_NAME to specify a node."
  echo "Use --help for more information."
  exit 1
fi


POD_NAME="aperf-pod-${NODE_NAME//[.]/-}"

# Create pod YAML as a variable
POD_YAML=$(cat << EOF
apiVersion: v1
kind: Pod
metadata:
  name: ${POD_NAME}
  labels:
    app: aperf
spec:
  nodeSelector:
    kubernetes.io/hostname: "${NODE_NAME}"
  containers:
  - name: aperf-runner
    image: ${APERF_IMAGE}
    securityContext:
      privileged: true
    command: ["/bin/sh", "-c"]
    args:
    - |
      set -e
      set -o pipefail

      echo -e "Copy async-profiler files..."
      cp -r /opt/async-profiler /tmp/aperf/ > /dev/null
      ln -sf /tmp/aperf/async-profiler/bin/asprof /usr/bin/asprof > /dev/null
      ln -sf /tmp/aperf/async-profiler/bin/jfrconv /usr/bin/jfrconv > /dev/null
      export LD_LIBRARY_PATH="/tmp/aperf/async-profiler/lib:${LD_LIBRARY_PATH}"
      
      echo -e "Starting Aperf recording execution..."
      echo "Run: /usr/bin/aperf record --tmp-dir="/tmp/aperf/profile"  -r aperf_record ${APERF_OPTIONS}"
      mkdir -p /tmp/aperf/profile 
      chmod -R 777 /tmp/aperf/profile
      /usr/bin/aperf record --tmp-dir="/tmp/aperf/profile" -r aperf_record ${APERF_OPTIONS}
      rm -rf  /tmp/aperf/profile /tmp/aperf/async-profiler
      echo "APerf record completed"

      echo -e "\nStarting Aperf report generation..."
      echo "Run: /usr/bin/aperf report -r aperf_record -n aperf_report"
      /usr/bin/aperf report -r aperf_record -n aperf_report
      echo "APerf report generation completed"

      echo -e "\nWaiting for files to be copied..."
      sleep 7200

    resources:
      requests:
        memory: "${MEMORY_REQUEST}"
        cpu: "${CPU_REQUEST}"
      limits:
        memory: "${MEMORY_LIMIT}"
        cpu: "${CPU_LIMIT}"
    volumeMounts:
    - mountPath: /boot
      name: boot-volume
      readOnly: true 
    - name: aperf-shared
      mountPath: /tmp/aperf
  volumes:
  - name: boot-volume
    hostPath:
      path: /boot
      type: Directory
  - name: opt
    hostPath:
      path: /opt
      type: DirectoryOrCreate
  - name: aperf-shared
    hostPath:
      path: /tmp/aperf
      type: DirectoryOrCreate
  hostPID: true
  hostNetwork: true
  restartPolicy: Never
EOF
)


# Get node instance type information
echo -e -n "${BOLD}Tageted node instance type... ${NC}  "
kubectl get node ${NODE_NAME}  -o jsonpath='{.metadata.labels.beta\.kubernetes\.io/instance-type}'

# Check if namespace has security restriction. If yes, exit.
ENFORCE=$(kubectl get namespace $NAMESPACE -o jsonpath='{.metadata.labels.pod-security\.kubernetes\.io/enforce}' 2>/dev/null)
echo -e -n "\n${BOLD}Check namespace security policy... ${NC}  "
if [ "$ENFORCE" = "baseline" ] || [ "$ENFORCE" = "restricted" ]; then
  echo "Namespace '$NAMESPACE' has '$ENFORCE' policy - privileged pods NOT allowed. Exit"
  exit 1
elif [ -z "$ENFORCE" ]; then
  echo "Namespace '$NAMESPACE' has no policy restrictions - privileged pods allowed."
else
  echo "Namespace '$NAMESPACE' has '$ENFORCE' policy - privileged pods allowed."
fi

# Show resource usage for pods on this node
echo -e "${BOLD}Resource usage for pods on ${NODE_NAME}:${NC}"
kubectl top pods --all-namespaces > /tmp/allpods.out  && \
head -n 1 /tmp/allpods.out  &&  \
grep "$(kubectl get pods --all-namespaces --field-selector spec.nodeName=${NODE_NAME} -o jsonpath='{range .items[*]}{.metadata.name}{" "}{end}' | sed 's/[[:space:]]*$//' | sed 's/[[:space:]]/\\|/g')" /tmp/allpods.out --color=never

# Create APerf pod
echo -e "\n${BOLD}Created pod configuration for node:${NC} ${NODE_NAME}${NC}"

# Delete existing pod if it exists
if kubectl get pod ${POD_NAME} -n ${NAMESPACE} &>/dev/null; then
  echo -e "${RED}Existing pod found. Delete it before continue.${NC}"
  echo -e "${RED}Possilbe command to use: kubectl delete pod ${POD_NAME} -n ${NAMESPACE} --force --grace-period=0 ${NC}"
  sleep 2
  exit
fi

# Apply the pod directly from variable
echo -e -n "${BOLD}Deploying pod to Kubernetes...${NC}  "
echo "$POD_YAML" | kubectl apply -f - -n ${NAMESPACE}

# Wait for pod to start
echo -e -n "${BOLD}Waiting for pod to start...${NC}  "
if ! kubectl wait --for=condition=ready pod/${POD_NAME} -n ${NAMESPACE} --timeout=60s; then
  echo -e "${RED}${BOLD}Pod failed to reach ready state. Showing pod logs:${NC}"
  kubectl describe pod ${POD_NAME} -n ${NAMESPACE}
  echo -e "\n${RED}${BOLD}Pod logs:${NC}"
  kubectl logs ${POD_NAME} -n ${NAMESPACE}
  echo -e "\n${RED}${BOLD}Cleaning up resources...${NC}"
  kubectl delete pod ${POD_NAME} -n ${NAMESPACE} --force --grace-period=0 2>/dev/null || true
  exit 1
fi

# Start recording time
POD_STARTTIME=$(date +%Y%m%d-%H%M%S)

# Start logs in background and save PID
echo -e "${BOLD}Starting program logs...${NC} ${YELLOW}"
kubectl logs -f ${POD_NAME} -n ${NAMESPACE} --tail=100 &
LOGS_PID=$!

# Wait until we see the "Waiting for files to be copied" message in the logs
while ! kubectl logs ${POD_NAME} -n ${NAMESPACE} | grep -q "Waiting for files to be copied"; do
  sleep 5s
done

# Kill the logs tail process
kill $LOGS_PID 2>/dev/null || true

# Copy files from pod to local directory
LOCAL_FILE="aperf_report_${POD_STARTTIME}.tar.gz"
echo -e "${NC}${BOLD}Aperf completed. Copying files from pod ${POD_NAME}...${NC}"
kubectl cp ${NAMESPACE}/${POD_NAME}:aperf_report.tar.gz ${LOCAL_FILE}

# Delete the pod after copying files
echo -ne "${BOLD}Deleting pod to clean up resources...${NC}  "
kubectl delete pod ${POD_NAME} -n ${NAMESPACE}

echo -e "${BOLD}${GREEN}Files copied to${NC} ${BLUE}${LOCAL_FILE}${NC}"
echo -e "${BOLD}${GREEN}Done!${NC}"
