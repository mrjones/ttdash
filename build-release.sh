version=$1

if [[ -z $1 ||  -z $2 ]]
then
    echo "Must pass major & minor version arguments. (Format: 20190721 1)";
    exit;
fi

echo "Building version [$1.$2]";

PATH=$PATH:/home/mrjones/src/pitools/arm-bcm2708/arm-linux-gnueabihf/bin/ OPENSSL_INCLUDE_DIR=/home/mrjones/arm/include OPENSSL_LIB_DIR=/home/mrjones/arm/lib TTDASH_VERSION="$1.$2" cargo build --target arm-unknown-linux-gnueabihf

binary=target/arm-unknown-linux-gnueabihf/debug/ttdash
checksum=$(md5sum $binary | awk '{print $1}')
cp $binary /var/www/html/ttdash-$1.$2

newBody=$(cat <<EOF
{
  "version": {
    "major": $1,
    "minor": $2
  },
  "md5sum": "${checksum}",
  "url": "http://linode.mrjon.es/ttdash-$1.$2"
}
EOF
)

cp /var/www/html/ttdash.version{,.bak}
echo $newBody > /var/www/html/ttdash.version
