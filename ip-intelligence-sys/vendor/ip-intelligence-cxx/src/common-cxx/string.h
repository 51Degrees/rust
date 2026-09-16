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

#ifndef FIFTYONE_DEGREES_STRING_H_INCLUDED
#define FIFTYONE_DEGREES_STRING_H_INCLUDED

/**
 * @ingroup FiftyOneDegreesCommon
 * @defgroup FiftyOneDegreesString String
 *
 * String structures containing the string and length.
 *
 * ## Introduction
 *
 * The String structure allows a string and its length to be stored in one
 * structure. This avoids unnecessary calls to strlen. Both the string and its
 * length are allocated in a single operation, so the size of the actual
 * structure (when including the string terminator) is
 * sizeof(#fiftyoneDegreesString) + length. This means that the string itself
 * starts at "value" and continues into the rest of the allocated memory.
 *
 * ## Get
 *
 * Getting a const char * from a #fiftyoneDegreesString structure can be done
 * by casting a reference to the "value" field:
 * ```
 * (const char*)&string->value
 * ```
 * However, this can be simplified by using the #FIFTYONE_DEGREES_STRING macro
 * which also performs a NULL check on the structure to avoid a segmentation
 * fault.
 *
 * ## Compare
 *
 * This file contains two case insensitive string comparison methods as
 * standards like `stricmp` vary across compilers.
 *
 * **fiftyoneDegreesStringCompare** : compares two strings case insensitively
 *
 * **fiftyoneDegreesStringCompareLength** : compares two strings case
 * insensitively up to the length required. Any characters after this point are
 * ignored
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

/**
 * Macro used to check for NULL before returning the string as a const char *.
 * @param s pointer to the #fiftyoneDegreesString
 * @return const char * string or NULL
 */
#define FIFTYONE_DEGREES_STRING(s) \
	(const char*)(s == NULL ? NULL : &((fiftyoneDegreesString*)s)->value)

/** 
 * String structure containing its value and size which maps to the string 
 * byte format used in data files.
 *
 * @example
 * String:
 * 			Short – length – 10
 * 			Byte value – first character of string – '5'
 */
#pragma pack(push, 1)
typedef struct fiftyone_degrees_string_t {
	int16_t size; /**< Size of the string in memory (starting from 'value') */
	char value; /**< The first character of the string */
} fiftyoneDegreesString;
#pragma pack(pop)

/**
 * Gets size of String with trailing characters.
 * @param initial pointer to string "head"
 * @return full (with tail) struct size
 */
#ifndef FIFTYONE_DEGREES_MEMORY_ONLY
EXTERNAL uint32_t fiftyoneDegreesStringGetFinalSize(
	const void *initial,
	fiftyoneDegreesException *exception);
#else
#define fiftyoneDegreesStringGetFinalSize NULL
#endif


#ifndef FIFTYONE_DEGREES_MEMORY_ONLY
/**
 * Reads a string from the source file at the offset within the string
 * structure.
 * @param file collection to read from
 * @param key of the string in the collection
 * @param data to store the new string in
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h.
 * @return a pointer to the string collection item or NULL if can't be found
 */
EXTERNAL void* fiftyoneDegreesStringRead(
	const fiftyoneDegreesCollectionFile *file,
	const fiftyoneDegreesCollectionKey *key,
	fiftyoneDegreesData *data,
	fiftyoneDegreesException *exception);

#endif

/**
 * Gets the string at the required offset from the collection provided.
 * @param strings collection to get the string from
 * @param offset of the string in the collection
 * @param item to store the string in
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h.
 * @return a pointer to string of NULL if the offset is not valid
 */
EXTERNAL const fiftyoneDegreesString* fiftyoneDegreesStringGet(
	const fiftyoneDegreesCollection *strings,
	uint32_t offset,
	fiftyoneDegreesCollectionItem *item,
	fiftyoneDegreesException *exception);

/**
 * Case insensitively compare two strings up to the length requested.
 * @param a string to compare
 * @param b other string to compare
 * @param length of the strings to compare
 * @return 0 if same
 */
EXTERNAL int fiftyoneDegreesStringCompareLength(
	char const *a, 
	char const *b, 
	size_t length);

/**
 * Case insensitively compare two strings.
 * @param a string to compare
 * @param b other string to compare
 * @return 0 if same
 */
EXTERNAL int fiftyoneDegreesStringCompare(const char *a, const char *b);

/**
 * Case insensitively searching a first occurrence of a
 * substring.
 * @param a string to search
 * @param b substring to be searched for
 * @return pointer to the first occurrence or NULL if not found
 */
EXTERNAL const char *fiftyoneDegreesStringSubString(const char *a, const char *b);

/**
 * @}
 */

#include "stringBuilder.h"

#endif
