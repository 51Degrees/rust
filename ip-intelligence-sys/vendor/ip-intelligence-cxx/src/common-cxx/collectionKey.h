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

#ifndef FIFTYONE_DEGREES_COLLECTION_KEY_H_INCLUDED
#define FIFTYONE_DEGREES_COLLECTION_KEY_H_INCLUDED

/**
 * @ingroup FiftyOneDegreesCommon
 * @defgroup FiftyOneDegreesCollectionKey CollectionKey
 *
 * Group of related items such as keys.
 *
 * @{
 */

#include <stdint.h>
#include "propertyValueType.h"
#include "exceptions.h"

/**
 * Passed a pointer to the first part of a variable size item and returns
 * the size of the entire item.
 * @param initial pointer to the start of the item
 * @return size of the item in bytes
 */
typedef uint32_t (*fiftyoneDegreesCollectionGetVariableSizeMethod)(
	const void *initial,
	fiftyoneDegreesException *exception);

/**
 * Location of the item within the Collection.
 */
typedef union fiftyone_degrees_collection_index_or_offset_t {
	uint32_t index;  /**< index of the item in the collection. */
	uint32_t offset;  /**< byte offset of the item from the start of collection. */
} fiftyoneDegreesCollectionIndexOrOffset;

static const fiftyoneDegreesCollectionIndexOrOffset
	fiftyoneDegreesCollectionIndexOrOffset_Zero = { 0 };

/**
 * Explains to a collection how to properly extract the requested value.
 */
typedef struct fiftyone_degrees_collection_key_type_t {
	const fiftyoneDegreesPropertyValueType valueType;  /**< Size of known-length "head" of the item. */
	uint32_t initialBytesCount; /**< Size of known-length "head" of the item. */
	const fiftyoneDegreesCollectionGetVariableSizeMethod getFinalSizeMethod; /**< Size of unknown-length "tail" of the item. */
} fiftyoneDegreesCollectionKeyType;

/**
 * Explains to a collection (or cache) what the consumer is looking for.
 */
typedef struct fiftyone_degrees_collection_key_t {
	fiftyoneDegreesCollectionIndexOrOffset indexOrOffset; /**< Where to look for the item. */
	const fiftyoneDegreesCollectionKeyType *keyType;  /**< Not used if collection is fixed width. */
} fiftyoneDegreesCollectionKey;

/**
 * @}
 */

#endif
