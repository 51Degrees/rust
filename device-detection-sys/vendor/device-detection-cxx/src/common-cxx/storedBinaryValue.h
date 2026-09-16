/* *********************************************************************
 * This Original Work is copyright of 51 Degrees Mobile Experts Limited.
 * Copyright 2026 51 Degrees Mobile Experts Limited, Davidson House,
 * Forbury Square, Reading, Berkshire, United Kingdom RG1 3EU.
 *
 * This Original Work is licensed under the European Union Public Licence
 * (EUPL) v.1.2 and is subject to its terms as set out below.
 *
 * If a copy of the EUPL was not distributed with this file, You can obtain
 * one at https://opensource.org/licenses/EUPL-1.2.
 *
 * The 'Compatible Licences' set out in the Appendix to the EUPL (as may be
 * amended by the European Commission) shall be deemed incompatible for
 * the purposes of the Work and the provisions of the compatibility
 * clause in Article 5 of the EUPL shall not apply.
 *
 * If using the Work as, or as part of, a network application, by
 * including the attribution notice(s) required under Article 5 of the EUPL
 * in the end user terms of the application under an appropriate heading,
 * such notice(s) shall fulfill the requirements of that article.
 * ********************************************************************* */

#ifndef FIFTYONE_DEGREES_STORED_BINARY_VALUE_H_INCLUDED
#define FIFTYONE_DEGREES_STORED_BINARY_VALUE_H_INCLUDED

/**
 * @ingroup FiftyOneDegreesCommon
 * @defgroup FiftyOneDegreesString String
 *
 * Byte array structures containing raw data bytes.
 *
 * @{
 */

#include <stdint.h>
#include <ctype.h>
#include "exceptions.h"
#include "collection.h"
#include "float.h"
#include "common.h"
#include "ip.h"
#include "propertyValueType.h"
#include "string.h"

/**
 * Structure containing raw bytes and size from data files.
 *
 * @example
 * String:
 * 			Short – length – 10
 * 			Byte value – first character of string – '5'
 * @example
 * Byte array:
 * 			Short – length – 3
 * 			Byte[] – bytes – [ 1, 2 ]
 * @example
 * IP (v4) address:
 * 			Short – length – 5
 * 			Byte[] – IP – [ 1, 2, 3, 4 ]
 * @example
 * WKB (value of  POINT(2.0 4.0)):
 * 			Short – length - 21
 * 			Byte[] – value – [
 * 				0 (endianness),
 * 				0, 0, 0, 1 (2D point),
 * 				128, 0, 0, 0, 0, 0, 0, 0 (2.0 float),
 * 				128, 16, 0, 0, 0, 0, 0, 0 (4.0 float)
 * 			]
 */
#pragma pack(push, 1)
typedef struct fiftyone_degrees_var_length_byte_array_t {
 int16_t size; /**< Size of the byte array in memory (starting from 'firstByte') */
 unsigned char firstByte; /**< The first byte of the array */
} fiftyoneDegreesVarLengthByteArray;
#pragma pack(pop)

/**
 * "Packed" value that can be present inside "strings" of dataset.
 */
#pragma pack(push, 1)
typedef union fiftyone_degrees_stored_binary_value_t {
 fiftyoneDegreesString stringValue; /**< String value (ASCII or UTF-8) */
 fiftyoneDegreesVarLengthByteArray byteArrayValue; /**< Byte array value (e.g. IP or WKB) */
 fiftyoneDegreesFloat floatValue; /**< single precision floating point value */
 int32_t intValue; /**< Integer value */
 int16_t shortValue; /**< Short value. Potentially half(-precision float). */
 byte byteValue; /**< Single byte value. */
} fiftyoneDegreesStoredBinaryValue;
#pragma pack(pop)

#ifndef FIFTYONE_DEGREES_MEMORY_ONLY

/**
 * Reads a binary value from the source file at the offset within the string
 * structure.
 * @param file collection to read from
 * @param key of the binary value in the collection
 * @param data to store the new string in
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h.
 * @return a pointer to the string collection item or NULL if can't be found
 * @note expects `data` to contain `fiftyoneDegreesPropertyValueType`
 * matching the stored value type of the property this value belongs to.
 */
EXTERNAL void* fiftyoneDegreesStoredBinaryValueRead(
 const fiftyoneDegreesCollectionFile *file,
 const fiftyoneDegreesCollectionKey *key,
 fiftyoneDegreesData *data,
 fiftyoneDegreesException *exception);

#endif

/**
 * Gets the binary value at the required offset from the collection provided.
 * @param strings collection to get the string from
 * @param offset of the binary value in the collection
 * @param storedValueType format of byte array representation
 * @param item to store the string in
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h.
 * @return a pointer to binary value or NULL if the offset is not valid
 */
EXTERNAL const fiftyoneDegreesStoredBinaryValue* fiftyoneDegreesStoredBinaryValueGet(
 const fiftyoneDegreesCollection *strings,
 uint32_t offset,
 fiftyoneDegreesPropertyValueType storedValueType,
 fiftyoneDegreesCollectionItem *item,
 fiftyoneDegreesException *exception);

/**
 * Function to compare the current binary value to the
 * target string value using the text format.
 * @param value the current binary value item
 * @param storedValueType format of byte array representation
 * @param target the target search value.
 * @param tempBuilder temporary builder to stringify value into.
 * @return 0 if they are equal, otherwise negative
 * for smaller and positive for bigger
 */
EXTERNAL int fiftyoneDegreesStoredBinaryValueCompareWithString(
 const fiftyoneDegreesStoredBinaryValue *value,
 fiftyoneDegreesPropertyValueType storedValueType,
 const char *target,
 fiftyoneDegreesStringBuilder *tempBuilder,
 fiftyoneDegreesException *exception);

/**
 * Function to convert the binary value to int when possible.
 * @param value the current binary value item
 * @param storedValueType format of byte array representation
 * @param defaultValue fallback result.
 * @return converted value (when possible)
 * or default one (when type is not convertible).
 */
EXTERNAL int fiftyoneDegreesStoredBinaryValueToIntOrDefault(
 const fiftyoneDegreesStoredBinaryValue *value,
 fiftyoneDegreesPropertyValueType storedValueType,
 int defaultValue);

/**
 * Function to convert the binary value to double when possible.
 * @param value the current binary value item
 * @param storedValueType format of byte array representation
 * @param defaultValue fallback result.
 * @return converted value (when possible)
 * or default one (when type is not convertible).
 */
EXTERNAL double fiftyoneDegreesStoredBinaryValueToDoubleOrDefault(
 const fiftyoneDegreesStoredBinaryValue *value,
 fiftyoneDegreesPropertyValueType storedValueType,
 double defaultValue);

/**
 * Function to convert the binary value to bool when possible.
 * @param value the current binary value item
 * @param storedValueType format of byte array representation
 * @param defaultValue fallback result.
 * @return converted value (when possible)
 * or default one (when type is not convertible).
 */
EXTERNAL bool fiftyoneDegreesStoredBinaryValueToBoolOrDefault(
 const fiftyoneDegreesStoredBinaryValue *value,
 fiftyoneDegreesPropertyValueType storedValueType,
 bool defaultValue);

/**
 * @}
 */

#endif
