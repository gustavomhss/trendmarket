from setuptools import setup

setup(
    name="PyYAML",
    version="0.0.0-local",
    description="Minimal YAML loader for offline use",
    packages=["yaml"],
    package_dir={'': '.'},
    python_requires=">=3.8",
)
