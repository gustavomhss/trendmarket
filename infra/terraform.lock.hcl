# This lockfile pins Terraform providers for deterministic pipelines.
provider "registry.terraform.io/hashicorp/aws" {
  version = "5.45.0"
  hashes = [
    "h1:zKt4zYgJpSfbAACTE2vU2Vq5uGN2W4kmZ4Mj9H1Np0w=",
  ]
}

provider "registry.terraform.io/hashicorp/random" {
  version = "3.6.3"
  hashes = [
    "h1:ZxV9H9Ckdrdr1UNpSUpWPidMpwVg0ak7X2yz16P5SJM=",
  ]
}
