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

/**
 * @ingroup FiftyOneDegreesCommon
 * @defgroup FiftyOneDegreesFile File
 *
 * File handle pool and simple file operations e.g. copy and delete.
 *
 * ## Introduction
 *
 * Implements a pool of file handles for use within multi-threaded environments
 * where the overhead of opening and closing a file handle for each thread
 * would be too great. Primarily used to load collection items from file with 
 * file based collections or where a cache is used.
 *
 * ## Creation
 *
 * The #fiftyoneDegreesFilePoolInit method is used to initialise a pointer to
 * a #fiftyoneDegreesFilePool. A concurrency value is provided to indicate the
 * maximum number of threads that will be in operation. If this value is lower
 * than the actual number of threads the stack can be exhausted and a null
 * pointer is returned instead of a valid file handle. The concurrency value
 * must always be the same or greater than the number of threads. When compiled
 * in single threaded operation a pool is not strictly required and the
 * implementation maintains a simple stack for consistency of interface and to
 * minimise divergent code.
 *
 * ## Get & Release
 *
 * Handles are retrieved from the pool via the #fiftyoneDegreesFileHandleGet 
 * method. The handle **MUST** be returned with the 
 * #fiftyoneDegreesFileHandleRelease method when it is finished with. The
 * handle will always be open and ready for read only operation. The position
 * of the handle within the source file cannot be assumed. If too many threads
 * are accessing the pool simultaneously, meaning a handle cannot be secured,
 * then a NULL pointer is returned.
 *
 * ## Free
 *
 * The handles are closed when the reader is released via the
 * #fiftyoneDegreesFilePoolRelease method. Any memory allocated by the 
 * implementation for the stack is freed.
 *
 * ## File Operations
 *
 * Common file operations can also be carried out using the methods defined
 * here. The supported operations are:
 *
 * **copy** : #fiftyoneDegreesFileCopy
 *
 * **create directory** : #fiftyoneDegreesFileCreateDirectory
 *
 * **create temp file** : #fiftyoneDegreesFileCreateTempFile
 *
 * **delete** : #fiftyoneDegreesFileDelete
 *
 * **get existing temp file** : #fiftyoneDegreesFileGetExistingTempFile
 *
 * **get file name** : #fiftyoneDegreesFileGetFileName
 *
 * **get path** : #fiftyoneDegreesFileGetPath
 *
 * **get size** : #fiftyoneDegreesFileGetSize
 *
 * **open** : #fiftyoneDegreesFileOpen
 *
 * **read to byte array** : #fiftyoneDegreesFileReadToByteArray
 *
 * **write** : #fiftyoneDegreesFileWrite
 *
 * ## Usage Example
 *
 * ```
 * FIFTYONE_DEGREES_EXCEPTION_CREATE;
 * fiftyoneDegreesFilePool pool;
 * const char *fileName;
 *
 * // Initialise a file pool
 * fiftyoneDegreesStatus status = fiftyoneDegreesFilePoolInit(
 *     &filePool,
 *     fileName,
 *     1,
 *     exception);
 *
 * // Check that the pool was initialised successfully
 * if (status == FIFTYONE_DEGREES_STATUS_SUCCESS) {
 *
 *     // Get a file handle from the pool
 *     fiftyoneDegreesFileHandle *handle = fiftyoneDegreesFileHandleGet(
 *         &pool,
 *         exception);
 *
 *     // Check that the handle was returned successfully
 *     if (FIFTYONE_DEGREES_EXCEPTION_OKAY) {
 *
 *         // Do something with the file pointer in handle->file
 *         // ...
 *
 *         // Release the file handle back to the pool
 *         fiftyoneDegreesFileHandleRelease(handle);
 *     }
 *
 *     // Release the file pool
 *     fiftyoneDegreesFilePoolRelease(&pool);
 * }
 * ```
 *
 * ## Design
 *
 * To improve performance in multi-threaded operation a non locking stack is 
 * used where a Compare and Swap (CAS) atomic operation is used to pop and push 
 * handles on and off the stack. The design was adapted from the following 
 * article (http://nullprogram.com/blog/2014/09/02/) which explains some of the 
 * challenges involved including the ABA problem 
 * (https://en.wikipedia.org/wiki/ABA_problem). It is for this reason the head 
 * structure is implemented as a union between the values and the exchange
 * integer. Pointers are not used as the address space for the stack is 
 * continuous and always very small compared to the total addressable memory 
 * space.
 *
 * @{
 */

#ifndef FIFTYONE_DEGREES_FILE_H_INCLUDED
#define FIFTYONE_DEGREES_FILE_H_INCLUDED

/* Define NDEBUG if needed, to ensure asserts are disabled in release builds */
#if !defined(DEBUG) && !defined(_DEBUG) && !defined(NDEBUG)
#define NDEBUG
#endif

#include <limits.h>
#include <stdio.h>
#include <errno.h>
#include <stdint.h>
#include <stdbool.h>
#ifdef _MSC_VER
#pragma warning (push)
#pragma warning (disable: 5105) 
#include <windows.h>
#pragma warning (default: 5105) 
#pragma warning (pop)
#include <direct.h>
#include <tchar.h>
#else
#include <dirent.h>
#include <sys/stat.h>
#endif
#include <time.h>
#include <assert.h>
#include <limits.h>
#include <time.h>
#include "data.h"
#include "fileOffset.h"
#include "exceptions.h"
#include "status.h"
#include "memory.h"
#include "pool.h"
#include "threading.h"
#include "common.h"

/**
 * Platform specific method pointer to return the current working directory.
 */
#ifdef _MSC_FULL_VER
#define GET_CURRENT_DIR _getcwd
#pragma warning (push)
#pragma warning (disable: 5105) 
#include <windows.h>
#pragma warning (default: 5105) 
#pragma warning (pop)
#include <direct.h>
#include <tchar.h>
#else
#define GET_CURRENT_DIR getcwd
#include <unistd.h>
#include <dirent.h>
#include <sys/stat.h>
#endif

#define TEMP_UNIQUE_STRING_LENGTH 20

/**
 * Define the max path length on the target system. For Windows this is 
 * `260` characters, and `4096` on most UNIX systems. The compiler defined
 * values are not used here as although Windows MSVC always defined
 * `_MAX_PATH`, UNIX systems do not consistently define the `PATH_MAX` macro
 * resulting in undefined behavior. For this reason the length of `4096` is
 * used as this is almost always the case.
 */
#ifdef _MSC_FULL_VER
#define FIFTYONE_DEGREES_FILE_MAX_PATH 260
#else
#define FIFTYONE_DEGREES_FILE_MAX_PATH 4096
#endif


/**
 * File handle node in the stack of handles.
 */
typedef union fiftyone_degrees_file_handle_t {
	FILE *file; /**< Open read handle to the source data file. */
	fiftyoneDegreesPoolItem item; /**< The pool item with the resource. */
} fiftyoneDegreesFileHandle;

/**
 * Stack of handles used to read data from a single source file.
 */
 typedef struct fiftyone_degrees_file_pool_t {
	 fiftyoneDegreesPool pool; /**< The pool of file handles */
	 fiftyoneDegreesFileOffset length; /**< Length of the file in bytes */
} fiftyoneDegreesFilePool;

/**
 * Moves the file pointer to a specified location.
 * @param stream Pointer to FILE structure.
 * @param offset Number of bytes from origin.
 * @param origin Initial position.
 * @return 
 */
EXTERNAL int fiftyoneDegreesFileSeek(
   FILE *stream,
   fiftyoneDegreesFileOffset offset,
   int origin);

/**
 * Gets the current position of a file pointer.
 * @param stream Target FILE structure.
 * @return The current file position.
 */
EXTERNAL fiftyoneDegreesFileOffset fiftyoneDegreesFileTell(
   FILE *stream
);

/**
 * Releases the file handles contained in the pool and frees any internal
 * memory used by the pool. Does not free the memory pointed to by pool.
 * @param pool pointer to the stack of file handles to be release
 */
EXTERNAL void fiftyoneDegreesFilePoolRelease(fiftyoneDegreesFilePool* pool);

/**
 * Opens the file path provided placing the file handle in the handle
 * parameter.
 * @param fileName full path to the file to open
 * @param handle to be associated with the open file
 * @return the result of the open operation
 */
EXTERNAL fiftyoneDegreesStatusCode fiftyoneDegreesFileOpen(
	const char* fileName,
	FILE** handle);

/**
 * Writes binary data to the file path provided, closing the file once finished.
 * @param fileName full path to the file to write to
 * @param data binary data to write
 * @param length the number of bytes in the data to be written
 * @return the result of the write operation
 */
EXTERNAL fiftyoneDegreesStatusCode fiftyoneDegreesFileWrite(
	const char* fileName,
	const void *data,
	const size_t length);

/**
 * Copy a file from one location to another. If there was an error while
 * creating or copying the file, then the appropriate status code will be
 * returned. If the status code is anything other than
 * #FIFTYONE_DEGREES_STATUS_SUCCESS then the new file will not exist.
 * @param source path to the file to copy
 * @param destination path to the file to create
 * @return the result of the copy operation
 */
EXTERNAL fiftyoneDegreesStatusCode fiftyoneDegreesFileCopy(
	const char *source,
	const char *destination);

/**
 * Delete a file from the file system. If there was an error while removing the
 * file, then the appropriate status code will be returned.
 * @param fileName path to the file to be deleted
 * @return the result of the delete operation 
 */
EXTERNAL fiftyoneDegreesStatusCode fiftyoneDegreesFileDelete(
	const char *fileName);

/**
 * Creates a directory with the specified path, and returns the result of the
 * operation. intermediate directories will not be created.
 * @param pathName path to create
 * @return the result of the operation
 */
EXTERNAL fiftyoneDegreesStatusCode fiftyoneDegreesFileCreateDirectory(
	const char *pathName);

/**
 * Iterates up the folders from the current working directory until a file
 * in the sub folder dataFolderName with the name fileName is found which
 * can be opened. This is assumed to be the data file required by the caller.
 * Enough memory must be allocated to the destination parameter for the full
 * path to be written. The maximum path length is defined by the macro
 * `FIFTYONEDEGREES_FILE_MAX_PATH`.
 * @param dataFolderName the name of the sub folder which is expected to 
 * contain the data file
 * @param fileName the name of the data file
 * @param destination memory to write the absolute path to if the file was
 * found
 * @param size the number of bytes assigned to the destination pointer
 * @return the result of the operation
 */
EXTERNAL fiftyoneDegreesStatusCode fiftyoneDegreesFileGetPath(
	const char *dataFolderName,
	const char *fileName,
	char *destination,
	size_t size);

/**
 * Gets the path to a temporary file which is an exact copy of the master file
 * if one exists. If one is found, true is returned and the path written to the
 * memory pointed to by the destination parameter. If the file found has the
 * same name as the master file, then false is returned to avoid the same file
 * being treated as if it were a copy by external code.
 * If no paths are provided, then the working directory is searched.
 * @param masterFile path to the master file to find a temp version of
 * @param paths list of paths to search in order of preference
 * @param count number of paths in the array
 * @param bytesToCompare number of from the start of the file to compare for
 * equality with the master file, or -1 to compare the whole file
 * @param destination memory to write the found file path to
 * @return true if a copy of the master file was found, and its path written to
 * destination
 */
EXTERNAL bool fiftyoneDegreesFileGetExistingTempFile(
	const char *masterFile,
	const char **paths,
	int count,
	fiftyoneDegreesFileOffset bytesToCompare,
	const char *destination);

/**
 * Finds all the temporary files which is an exact copy of the master file
 * if any exist. If any are found, the method attempts to delete them. The
 * number of successfully deleted temp files is returned. If any files found
 * have the same name as the master file, then false is returned to avoid the
 * same file being treated as if it were a copy by external code.
 * If no paths are provided, then the working directory is searched. Note that
 * this should not be used on Apple systems, as the checks for whether or not
 * a file is in use are not implemented (all files will be deleted regardless
 * of whether they are being used).
 * @param masterFileName path to the master file to find a temp version of
 * @param paths list of paths to search in order of preference
 * @param count number of paths in the array
 * @param bytesToCompare number of from the start of the file to compare for
 * equality with the master file, or -1 to compare the whole file
 * @return the number of matching files which have been successfully deleted
 */
EXTERNAL int fiftyoneDegreesFileDeleteUnusedTempFiles(
	const char *masterFileName,
	const char **paths,
	int count,
	fiftyoneDegreesFileOffset bytesToCompare);

/**
 * Create a temporary file name and add it to the destination.
 * @param masterFileName basename of the master file
 * @param destination buffer to write the temporary file name to
 * @param nameStart position of where the name should be added to the destination
 * @param length length available in the buffer
 * @return the result of the file name creation
 */
EXTERNAL fiftyoneDegreesStatusCode fiftyoneDegreesFileAddTempFileName(
	const char* masterFileName,
	char* destination,
	size_t nameStart,
	size_t length);

/**
 * Create a temporary file containing a copy of the master file using the first
 * writable path in the list of paths provided. The path which is written to
 * (including file name) is written to the destination parameter which must
 * contain enough memory.
 * If no paths are provided, then the working directory used as the destination.
 * @param masterFile full path to the file containing the master data to copy
 * @param paths list of paths to use in order of preference
 * @param count number of paths in the array
 * @param destination memory to write the new file path to
 * @param length size of the memory to be written to
 * @return the result of the create operation
 */
EXTERNAL fiftyoneDegreesStatusCode fiftyoneDegreesFileNewTempFile(
	const char* masterFile,
	const char** paths,
	int count,
	char* destination,
	size_t length);

/**
 * [DEPRECATED - Use #fiftyoneDegreesFileNewTempFile instead]
 * Create a temporary file containing a copy of the master file using the first
 * writable path in the list of paths provided. The path which is written to
 * (including file name) is written to the destination parameter which must
 * contain enough memory.
 * If no paths are provided, then the working directory used as the destination.
 * @param masterFile full path to the file containing the master data to copy
 * @param paths list of paths to use in order of preference
 * @param count number of paths in the array
 * @param destination memory to write the new file path to
 * @return the result of the create operation
 */
EXTERNAL DEPRECATED fiftyoneDegreesStatusCode fiftyoneDegreesFileCreateTempFile(
	const char *masterFile,
	const char **paths,
	int count,
	const char *destination);

/**
 * Initialises the pool with a stack of open read only file handles all 
 * associated with the file name. The concurrency parameter determines the 
 * number of items in the stack.
 * @param filePool to be initialised
 * @param fileName full path to the file to open
 * @param concurrency number of items in the stack
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h.
 * @return the result of the open operation
 */
EXTERNAL fiftyoneDegreesStatusCode fiftyoneDegreesFilePoolInit(
	fiftyoneDegreesFilePool *filePool,
	const char *fileName,
	uint16_t concurrency,
	fiftyoneDegreesException *exception);

/**
 * Retrieves a read only open file handle from the pool. The handle retrieve
 * must be returned to the pool using #fiftyoneDegreesFileHandleGet and must
 * not be freed or closed directly.
 * @param filePool to retrieve the file handle from
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h.
 * @return a read only open file handle or NULL if no available handles remain
 * in the pool
 */
EXTERNAL fiftyoneDegreesFileHandle* fiftyoneDegreesFileHandleGet(
	fiftyoneDegreesFilePool *filePool,
	fiftyoneDegreesException *exception);

/**
 * Returns a handle previously retrieved via #fiftyoneDegreesFileHandleGet back
 * to the pool.
 * @param handle to be returned to the pool
 */
EXTERNAL void fiftyoneDegreesFileHandleRelease(
	fiftyoneDegreesFileHandle* handle);

/**
 * Returns the size of a file in bytes, or -1 if the file does not exist or
 * cannot be accessed.
 * @param fileName path to the file
 * @return size of file in bytes or -1
 */
EXTERNAL fiftyoneDegreesFileOffset fiftyoneDegreesFileGetSize(const char *fileName);

/**
 * Reads the contents of a file into memory. The correct amount of memory will
 * be allocated by the method. This memory needs to be freed by the caller
 * after the data has been finished with.
 * @param fileName path to the source file
 * @param reader to contain the pointer to the memory and the size
 * @return status code indicating whether the read was successful
 */
EXTERNAL fiftyoneDegreesStatusCode fiftyoneDegreesFileReadToByteArray(
	const char *fileName,
	fiftyoneDegreesMemoryReader *reader);

/**
 * Resets the pool without releasing any resources.
 * @param filePool to be reset.
 */
EXTERNAL void fiftyoneDegreesFilePoolReset(fiftyoneDegreesFilePool *filePool);

/**
 * Gets the last, file name, segment of the full file path.
 * @param filePath full path to the file.
 * @return the file name from the file path.
 */
EXTERNAL const char* fiftyoneDegreesFileGetFileName(const char *filePath);

/**
 * @}
 */
#endif