import { Construct } from "constructs";
import { App, TerraformStack, TerraformOutput } from "cdktf";
import * as path from "path";
import * as fs from "fs";
import { execSync } from "child_process";
import { config } from "dotenv";
import { GoogleProvider } from "./.gen/providers/google/provider";
import { ComputeNetwork } from "./.gen/providers/google/compute-network";
import { ComputeInstance } from "./.gen/providers/google/compute-instance";
import { ComputeFirewall } from "./.gen/providers/google/compute-firewall";

config();

class MyStack extends TerraformStack {
    constructor(scope: Construct, name: string) {
        super(scope, name);

        const credentialsPath = path.join(process.cwd(), "google.json");
        const credentials = fs.existsSync(credentialsPath)
            ? fs.readFileSync(credentialsPath).toString()
            : "{}";

        // Generate SSH key pair if it doesn't exist
        const sshKeyPath = path.join(process.cwd(), "cdktf-ssh-key");
        if (!fs.existsSync(sshKeyPath)) {
            execSync(`ssh-keygen -t rsa -b 4096 -f ${sshKeyPath} -N "" -C "cdktf-user"`, {
                stdio: "inherit",
            });
        }

        // Read the public key
        const publicKey = fs.readFileSync(`${sshKeyPath}.pub`, "utf-8").trim();

        // Read vm_assets files
        const vmAssetsDir = path.join(__dirname, "vm_assets");

        const composeYml = fs.readFileSync(
            path.join(vmAssetsDir, "compose.yml"),
            "utf-8"
        );

        // Create Ignition config for CoreOS
        const ignitionConfig = {
            ignition: { version: "3.3.0" },
            passwd: {
                users: [
                    {
                        name: "core",
                        sshAuthorizedKeys: [publicKey],
                    },
                ],
            },
            storage: {
                files: [
                    {
                        path: "/var/home/core/compose.yaml",
                        mode: 0o644,
                        user: {
                            name: "core",
                        },
                        group: {
                            name: "core",
                        },
                        contents: {
                            source: `data:,${encodeURIComponent(composeYml)}`,
                        },
                    },
                ],
            },
            systemd: {
                units: [
                    {
                        name: "podman.socket",
                        enabled: true,
                        contents: `[Unit]
Description=Podman API Socket
Documentation=man:podman-system-service(1)

[Socket]
ListenStream=/run/podman/podman.sock
SocketMode=0660

[Install]
WantedBy=sockets.target`,
                    },
                    {
                        name: "install-podman-compose.service",
                        enabled: true,
                        contents: `[Unit]
Description=Download Docker Compose CLI plugin
After=network-online.target
Wants=network-online.target

[Service]
Type=oneshot
RemainAfterExit=yes
User=root
Group=0
ExecStartPre=/usr/bin/mkdir -p /usr/local/lib/docker/cli-plugins
ExecStart=/usr/bin/curl -SL "https://github.com/docker/compose/releases/download/v2.39.4/docker-compose-linux-aarch64" -o /usr/local/lib/docker/cli-plugins/docker-compose
ExecStartPost=/usr/bin/chmod +x /usr/local/lib/docker/cli-plugins/docker-compose`,
                    },
                    {
                        name: "start-services.service",
                        enabled: true,
                        contents: `[Unit]
Description=Spawn services with podman compose.
After=install-podman-compose.service podman.socket
Requires=install-podman-compose.service podman.socket

[Service]
Type=oneshot
RemainAfterExit=yes
WorkingDirectory=/home/core
User=root
ExecStart=/usr/bin/podman compose --file /var/home/core/compose.yaml up -d
ExecStop=/usr/bin/podman compose --file /var/home/core/compose.yaml down

[Install]
WantedBy=multi-user.target`,
                    },
                ],
            },
        };

        new GoogleProvider(this, "Google", {
            region: process.env.GCP_REGION || "us-central1",
            zone: process.env.GCP_ZONE || "us-central1-c",
            project: process.env.GCP_PROJECT_ID || "your-project-id",
            credentials,
        });

        const network = new ComputeNetwork(this, "Network", {
            name: process.env.NETWORK_NAME || "cdktf-network",
        });

        new ComputeFirewall(this, "AllowSSH", {
            name: "allow-ssh",
            network: network.name,
            allow: [
                {
                    protocol: "tcp",
                    ports: ["22"],
                },
            ],
            sourceRanges: ["0.0.0.0/0"],
            targetTags: ["web"],
        });

        const servicePorts = [
            // process.env.CONTAINER_API_PORT || "3001",
            process.env.REPL_API_PORT || "3002",
            // process.env.SERVICE_REGISTRY_PORT || "3003",
            // process.env.REGISTRY_PORT || "5001",
        ];

        new ComputeFirewall(this, "AllowServices", {
            name: "allow-services",
            network: network.name,
            allow: [
                {
                    protocol: "tcp",
                    ports: servicePorts,
                },
            ],
            sourceRanges: ["0.0.0.0/0"],
            targetTags: ["web"],
        });

        const instance = new ComputeInstance(this, "ComputeInstance", {
            name: process.env.INSTANCE_NAME || "cdktf-instance",
            machineType: process.env.MACHINE_TYPE || "c4a-standard-2",

            // Enable Spot VM behavior (preemptible)
            scheduling: {
                preemptible: true,
                automaticRestart: false,
                onHostMaintenance: "TERMINATE",
            },

            bootDisk: {
                initializeParams: {
                    image: process.env.BOOT_IMAGE || "fedora-coreos-cloud/fedora-coreos-next-arm64",
                },
            },

            networkInterface: [
                {
                    network: network.name,
                    accessConfig: [{}],
                },
            ],

            tags: ["web", "dev"],

            metadata: {
                "user-data": JSON.stringify(ignitionConfig),
            },

            dependsOn: [network],
        });

        const externalIp = instance.networkInterface.get(0).accessConfig.get(0).natIp;

        new TerraformOutput(this, "instance_external_ip", {
            value: externalIp,
        });

        new TerraformOutput(this, "ssh_command", {
            value: `ssh -i cdktf-ssh-key core@\${${externalIp}}`,
        });

        new TerraformOutput(this, "ssh_key_path", {
            value: sshKeyPath,
        });

        new TerraformOutput(this, "repl_api_url", {
            value: `http://\${${externalIp}}:${process.env.REPL_API_PORT || "3002"}`,
            description: "REPL API endpoint",
        });

        new TerraformOutput(this, "startup_instructions", {
            value: `After deployment:\n1. Services will start automatically via systemd\n2. SSH: ssh -i cdktf-ssh-key core@\${${externalIp}}\n3. Check service status: systemctl status start-services.service\n4. View logs: journalctl -u start-services.service -f\n5. Services will be available at the URLs above once startup completes (allow ~2-3 minutes)`,
        });
    }
}

const app = new App();
new MyStack(app, "typescript-gcp");
app.synth();
