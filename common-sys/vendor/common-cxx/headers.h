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

#ifndef FIFTYONE_DEGREES_HEADERS_H_INCLUDED
#define FIFTYONE_DEGREES_HEADERS_H_INCLUDED

/**
 * @ingroup FiftyOneDegreesCommon
 * @defgroup FiftyOneDegreesHeaders Headers
 *
 * Common form of evidence in 51Degrees engines.
 *
 * ## Introduction
 *
 * HTTP headers are a common form of evidence, so required headers have their
 * own structure and methods. By storing the unique id of headers, storing
 * duplicates of the same header can be avoided. Duplicates can occur as a
 * result of different cases or prefixes e.g. `User-Agent`, `user-agent` and
 * `HTTP_user-agent` are all the same header.
 *
 * ## Creation
 *
 * A header structure is created using the #fiftyoneDegreesHeadersCreate
 * method. This takes a state and a method used to extract the unique headers
 * from the state. See the method description for more details.
 *
 * ## Get
 *
 * A header can be fetched using it's unique id with the
 * #fiftyoneDegreesHeadersGetHeaderFromUniqueId method.
 *
 * The index of a header in the unique headers structure can also be fetched
 * using the #fiftyoneDegreesHeaderGetIndex method.
 *
 * ## Free
 *
 * Once a headers structure is finished with, it is released using the
 * #fiftyoneDegreesHeadersFree method.
 *
 * ## Usage Example
 *
 * ```
 * fiftyoneDegreesHeadersGetMethod getHeaderId;
 * void *state;
 *
 * // Create the headers structure
 * fiftyoneDegreesHeaders *headers = fiftyoneDegreesHeadersCreate(
 *     false,
 *     state,
 *     getHeaderId,
 *     exception);
 *
 * // Get the index of a header
 * int index = fiftyoneDegreesHeaderGetIndex(
 *     headers,
 *     "user-agent",
 *     strlen("user-agent"));
 *
 * // Check that the header exists in the structure
 * if (index >= 0) {
 *
 *     // Do something with the header
 *     // ...
 * }
 *
 * // Free the headers structure
 * fiftyoneDegreesHeadersFree(headers);
 * ```
 *
 * @{
 */

#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>
#ifdef _MSC_FULL_VER
#include <string.h>
#else
#include <strings.h>
#define _stricmp strcasecmp
#define _strnicmp strncasecmp
#endif
#include "list.h"
#include "array.h"
#include "common.h"

#define FIFTYONE_DEGREES_PSEUDO_HEADER_SEP '\x1F' /** unit separator of headers
													and headers' values that
													form pseudo header and
													its evidence */

/**
 * The unique id for the header field string in the data set.
 */
typedef uint32_t fiftyoneDegreesHeaderID;

/**
 * Forward declaration of the header structure.
 */
typedef struct fiftyone_degrees_header_t fiftyoneDegreesHeader;

/**
 * Pointer to a header structure. Used in an array of related headers.
 */
typedef fiftyoneDegreesHeader* fiftyoneDegreesHeaderPtr;

/**
 * Array of header indexes.
 */
FIFTYONE_DEGREES_ARRAY_TYPE(
	fiftyoneDegreesHeaderPtr,
	);
typedef fiftyoneDegreesHeaderPtrArray fiftyoneDegreesHeaderPtrs;

/**
 * Structure for a header known to the corresponding data set.
 */
struct fiftyone_degrees_header_t {
	uint32_t index; /**< Index of the header in the array of all headers */
	const char* name; /**< Name of the header or pseudo header field as a
					       null terminated string */
	size_t nameLength; /**< Length of the name string excluding the terminating 
						null */
	fiftyoneDegreesHeaderID headerId; /**< Unique id in the data set for this 
									  full header */
	bool isDataSet; /**< True if the header originates from the data set and 
					the headerId is valid */
	fiftyoneDegreesHeaderPtrs* pseudoHeaders; /**< Array of pointers to
												related pseudo headers */
	fiftyoneDegreesHeaderPtrs* segmentHeaders; /**< Array of pointers to raw
												  headers that form this pseudo
												  header */
};

#define FIFTYONE_DEGREES_HEADERS_MEMBERS \
bool expectUpperPrefixedHeaders; /**< True if the headers structure should
								 expect input header to be prefixed with
								 'HTTP_' */

/**
 * Array of Headers which should always be ordered in ascending order of 
 * fullHeaderId.
 */
FIFTYONE_DEGREES_ARRAY_TYPE(
	fiftyoneDegreesHeader, 
	FIFTYONE_DEGREES_HEADERS_MEMBERS);

/**
 * Array of headers used to easily access and track the size of the array.
 */
typedef fiftyoneDegreesHeaderArray fiftyoneDegreesHeaders;

/**
 * Gets the unique id and name of the header at the requested index. The caller
 * must use COLLECTION_RELEASE on nameItem when finished with the result.
 * @param state pointer to data used by the method
 * @param index of the header to get
 * @param nameItem pointer to the collection item to populate with the name of
 * the header
 * @return unique id of the header
 */
typedef long(*fiftyoneDegreesHeadersGetMethod)(
	void *state,
	uint32_t index, 
	fiftyoneDegreesCollectionItem *nameItem);

/**
 * Creates a new headers instance configured with the unique HTTP names needed
 * from evidence. If the useUpperPrefixedHeaders flag is true then checks for 
 * the presence of HTTP headers will also include checking for HTTP_ as a
 * prefix to the header key. If header is a pseudo header, the indices of
 * actual headers that form this header will be constructed.
 *
 * @param useUpperPrefixedHeaders true if HTTP_ prefixes should be checked
 * @param state pointer used by getHeaderMethod to retrieve the header integer
 * @param get used to return the HTTP header unique integer
 * @param exception
 * @return a new instance of #fiftyoneDegreesHeaders ready to be used to filter 
 * HTTP headers.
 */
EXTERNAL fiftyoneDegreesHeaders* fiftyoneDegreesHeadersCreate(
	bool useUpperPrefixedHeaders,
	void *state,
	fiftyoneDegreesHeadersGetMethod get,
	fiftyoneDegreesException* exception);

/**
 * Provides the integer index of the HTTP header name, or -1 if there is no 
 * matching header.
 * @param headers structure created by #fiftyoneDegreesHeadersCreate
 * @param httpHeaderName of the header whose index is required
 * @param length number of characters in httpHeaderName
 * @return the index of the HTTP header name, or -1 if the name does not exist
 */
EXTERNAL int fiftyoneDegreesHeaderGetIndex(
	fiftyoneDegreesHeaders *headers,
	const char* httpHeaderName,
	size_t length);

/**
 * Gets a pointer to the header in the headers structure with a unique id
 * matching the one provided. If the headers structure does not contain a
 * header with the unique id, NULL will be returned.
 * This method assumes that the headers in the structure are unique, if they
 * are not, then the first matching header will be returned.
 * @param headers pointer to the headers structure to search
 * @param uniqueId id to search for
 * @return pointer to the matching header, or NULL
 */
EXTERNAL fiftyoneDegreesHeader* fiftyoneDegreesHeadersGetHeaderFromUniqueId(
	fiftyoneDegreesHeaders *headers,
    fiftyoneDegreesHeaderID uniqueId);

/**
 * Frees the memory allocated by the #fiftyoneDegreesHeadersCreate method.
 *
 * @param headers structure created by #fiftyoneDegreesHeadersCreate
 */
EXTERNAL void fiftyoneDegreesHeadersFree(fiftyoneDegreesHeaders *headers);

/**
 * Determines if the field is an HTTP header.
 * @param state results instance to check against
 * @param field name from the evidence pair to be checked
 * @param length of field string
 * @return true if the evidence relates to an HTTP header, otherwise false.
 */
EXTERNAL bool fiftyoneDegreesHeadersIsHttp(
	void *state,
	const char* field,
	size_t length);

/**
 * @}
 */

#endif
