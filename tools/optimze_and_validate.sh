#!/bin/bash

IN_DIR=""
OUT_DIR=""
FILTER="*.glb"
RPT="/tmp/output.txt"

while getopts "i:o:f:" arg
do
	case "${arg}" in
		i)
			IN_DIR=$OPTARG
			;;
		o)
			OUT_DIR=$OPTARG
			;;
		f)
			FILTER=$OPTARG
			;;
		*)
			echo "$0 -i <in dir> -o <out dir>"
			;;
	esac
done

echo "IN_DIR = $IN_DIR" 
echo "OUT_DIR = $OUT_DIR"
echo "IN_DIR = $IN_DIR" > $RPT
echo "OUT_DIR = $OUT_DIR" >> $RPT

ls ${IN_DIR}/${FILTER} | while read line 
do
	echo "***************" >> $RPT
	echo $line 
	echo $line >> $RPT
	gltf-transform inspect $line >> $RPT
	gltf-transform validate $line >> $RPT
	gltf-transform optimize --simplify true --texture-compress webp --weld true --simplify-error 0.01 --simplify-ratio 0.0 $line /tmp/temp.glb >> $RPT
	gltf-transform validate /tmp/temp.glb >> $RPT
	mv /tmp/temp.glb $OUT_DIR/$(basename -- "$line") >> $RPT
	echo " ***************" >> $RPT
done


