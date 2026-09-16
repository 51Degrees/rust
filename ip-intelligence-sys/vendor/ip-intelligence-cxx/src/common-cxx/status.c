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

#include "status.h"
#include "fiftyone.h"

typedef struct fiftyone_degrees_status_message {
	StatusCode status;
	const char *message;
} StatusMessage;

static StatusMessage messages[] = {
	{ SUCCESS,
		"The operation was successful."},
	{ INSUFFICIENT_MEMORY,
		"Insufficient memory allocated for the operation." },
	{ CORRUPT_DATA,
		"The data was not in the correct format. Check the data file '%s' is "
		"uncompressed." },
	{ INCORRECT_VERSION,
		"The data (file: '%s') is an unsupported version. Check you have the latest data "
		"and API." },
	{ FILE_NOT_FOUND,
		"The data file '%s' could not be found. Check the file path and that "
		"the program has sufficient read permissions." },
	{ FILE_BUSY,
		"The data file '%s' was busy." },
	{ FILE_FAILURE,
		"An unknown error occurred accessing the file '%s'. Check the file "
		"path and that the program has sufficient read permissions." },
	// `NOT_SET` should NOT return a value.
	// See `Status.Get_NotSetMessage` unit test.
	//
	// { NOT_SET,
	// 	"This status code should never be returned to the caller." },
	{ NULL_POINTER,
		"Null pointer to the existing dataset or memory location." },
	{ POINTER_OUT_OF_BOUNDS,
		"Allocated continuous memory containing 51Degrees data file appears "
		"to be smaller than expected. Most likely because the data file was "
		"not fully loaded into the allocated memory." },
	{ TOO_MANY_OPEN_FILES,
		"Too many file handles have been created during initialisation. "
		"Original data file path: '%s'."},
	{ REQ_PROP_NOT_PRESENT,
		"None of the properties requested could be found in the data file ('%s'), so "
		"no properties can be initialised. To initialise all available "
		"properties, set the field to null." },
	{ PROFILE_EMPTY,
		"The profile id related to an empty profile. As this just represents "
		"an empty profile, there is no profile which can be returned." },
	{ COLLECTION_FAILURE,
		"There was an error getting an item from a collection within the "
		"data set (file: '%s'). This is likely to be caused by too many concurrent "
		"operations. Increase the concurrency option in the collection "
		"configuration to allow more threads to access the collection "
		"simultaneously." },
	{ FILE_COPY_ERROR,
		"There was an error copying the source file ('%s') to the destination. "
		"Verify sufficient space is available at the destination." },
	{ FILE_EXISTS_ERROR,
		"The file or directory already exists so could not be created." },
	{ FILE_WRITE_ERROR,
		"Could not create some file with write permissions. "
		"Original data file path: '%s'." },
	{ FILE_READ_ERROR,
		"Could not read the file." },
	{ FILE_PERMISSION_DENIED,
		"Permission denied when opening some file. "
		"Original data file path: '%s'." },
	{ FILE_PATH_TOO_LONG,
		"The file path to the data file '%s' is longer than the memory available "
		"to store it. Use a shorter data file path." },
	{ FILE_END_OF_DOCUMENT,
		"End of a Yaml document read." },
	{ FILE_END_OF_DOCUMENTS,
		"End of Yaml documents read." },
	{ FILE_END_OF_FILE,
		"End of file." },
	{ ENCODING_ERROR,
		"There was an error encoding characters of the string. Ensure all "
		"characters are valid. File: '%s'." },
	{ INVALID_COLLECTION_CONFIG,
		"The configuration provided could not be used to create a valid "
		"collection. If a cached collection is included in the configuration "
		"this maybe caused by insufficient capacity for the concurrency."},
	{ INVALID_CONFIG,
		"The configuration provided was not valid, and has caused a failure "
		"while building the resource it configures." },
	{ INSUFFICIENT_HANDLES,
		"Insufficient handles available in the pool. Verify the pool has "
		"sufficient handles to support the maximum number of concurrent "
		"threads. This can be set when creating the resource containg the "
		"pool. Another way to avoid this is by using an in-memory "
		"configuration, which avoids using file handles completely, and "
		"removes any limit on concurrency. For info see "
		"https://51degrees.com/documentation/4.5/_device_detection__features__concurrent_processing.html?utm_source=code&utm_medium=comment&utm_campaign=common-cxx&utm_content=status.c&utm_term=insufficient_handles"},
	{ COLLECTION_INDEX_OUT_OF_RANGE,
		"Index used to retrieve an item from a collection was out of range." },
	{ COLLECTION_OFFSET_OUT_OF_RANGE,
		"Offset used to retrieve an item from a collection was out of range." },
	{ COLLECTION_FILE_SEEK_FAIL,
		"A seek operation on a file ('%s') failed." },
	{ COLLECTION_FILE_READ_FAIL,
		"A read operation on a file ('%s') failed." },
	{ INCORRECT_IP_ADDRESS_FORMAT,
		"The input IP address format is incorrect. Verify the input IP address "
		"string has correct format. If passing a byte array, verify the "
		"associated input data is also consistent." },
	{ TEMP_FILE_ERROR,
		"Error occurs during the creation of a temporary file."},
	{ INSUFFICIENT_CAPACITY,
		"Insufficient capacity of the array to hold all the items."},
    { INVALID_INPUT, "The input value is invalid: misformatted or semantically inconsistent."},
    { UNSUPPORTED_STORED_VALUE_TYPE, "Property's StoredValueType is not supported at this version."},
    { FILE_TOO_LARGE, "File size exceeds malloc capabilities."},
    { UNKNOWN_GEOMETRY, "Unsupported geometry type found in WKB."},
    { RESERVED_GEOMETRY, "Geometry type found in WKB is abstract or reserved."},
	{ NOT_IMPLEMENTED, "The method is not implemented. This is usually because "
		"a compile flag has been set which removes the functionality."},
};

static char defaultMessage[] = "Status code %i does not have any message text.";

const char* fiftyoneDegreesStatusGetMessage(
	fiftyoneDegreesStatusCode status,
	const char *fileName) {
	uint32_t i;
	size_t messageSize;
	StatusMessage *current;
	char *message = NULL;
	if (fileName == NULL) {
		fileName = "null";
	}
	
	for (i = 0; i < sizeof(messages) / sizeof(StatusMessage); i++) {
		current = &messages[i];
		if (current->status == status) {
			messageSize = strstr(current->message, "%s") ?
				// message + dataFile + '\0' - "%s"
				strlen(current->message) + strlen(fileName) - 1 :
				// message + '\0'
				strlen(current->message) + 1;
			message = (char*)Malloc(messageSize);
			if (message != NULL) {
				Snprintf(message, messageSize, current->message, fileName);
			}
			break;
		}
	}
	if( message == NULL) {
		messageSize = sizeof(defaultMessage) + 5;
		message = (char*)Malloc(messageSize);
		Snprintf(message, messageSize, defaultMessage, (int)status);
	}
	return message;
}
