version=$1

if [[ -z $1 ||  -z $2 ]]
then
    echo "Must pass major & minor version arguments. (Format: 20190721 1)";
    exit;
fi

echo "Building version [$1.$2]";

# ===

cargoBinary=target/arm-unknown-linux-gnueabihf/release/ttdash
track=arm

versionFile=/var/www/html/ttdash-${track}.version
servingBinaryShortFilename=ttdash-${track}.${1}.${2}
servingBinaryFullPath=/var/www/html/${servingBinaryShortFilename}

# ===

PATH=$PATH:/home/mrjones/src/pitools/arm-bcm2708/arm-linux-gnueabihf/bin/ OPENSSL_INCLUDE_DIR=/home/mrjones/arm/include OPENSSL_LIB_DIR=/home/mrjones/arm/lib TTDASH_VERSION="${1}.${2}" cargo build --target arm-unknown-linux-gnueabihf --release

checksum=$(md5sum ${cargoBinary} | awk '{print $1}')
cp ${cargoBinary} ${servingBinaryFullPath}

newBody=$(cat <<EOF
{
  "version": {
    "major": $1,
    "minor": $2
  },
  "md5sum": "${checksum}",
  "url": "http://linode.mrjon.es/${servingBinaryShortFilename}"
}
EOF
)

cp ${versionFile}{,.bak} | true
echo ${newBody} > ${versionFile}
