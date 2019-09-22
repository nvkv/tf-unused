provider "aws" {
  region = "us-east-1"
}

locals {
  important = "${var.boring_but_important_variable > 0 ? 1 : 0}"
}

data "aws_ami" "ubuntu" {
  most_recent = true

  filter {
    name   = "owner-alias"
    values = ["amazon"]
  }

  filter {
    name   = "name"
    values = ["amzn2-ami-hvm*"]
  }
}

resource "aws_instance" "web" {
  ami           = "${data.aws_ami.ubuntu.id}"
  instance_type = "t2.micro"
  count         = "${local.important}"

  tags {
    Name = "${var.instance_name}"
  }
}
