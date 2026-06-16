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

#ifndef FIFTYONE_DEGREES_TREE_INCLUDED
#define FIFTYONE_DEGREES_TREE_INCLUDED


/**
 * @ingroup FiftyOneDegreesCommon
 * @defgroup FiftyOneDegreesTree Tree
 *
 * Implementation of the classic red black binary tree.
 *
 * ## Introduction
 *
 * Implementation of a classic red black binary tree adapted to support the
 * result set structure used in the LRU cache. Several important considerations
 * should be noted with this implementation.
 *
 * 1. The maximum number of entries in the tree is known when the tree is
 *    created. All memory allocation is performed at initialisation.
 * 2. Once the tree is full it will remain full and never shrinks. The memory
 *    used is freed when the cache is freed.
 * 3. A node in the tree also contains other data such as the linked list
 *    pointers used to identify the first and last entry in the cache, and
 *    the cache data itself. See structure Node.
 * 4. The cache structure Cache contains special fields "empty"
 *    and "root". "Empty" is used in place of NULL to indicate that the left,
 *    right or parent pointer of the node has no data. The use of "empty" makes
 *    the algorithm more efficient as the data structure used to indicate no
 *    data is the same as a valid data structure and therefore does not require
 *    custom logic. The "root" fields left pointer is used as the start of the
 *    tree. Similarly the parent element being a valid data structure
 *    simplifies the algorithm.
 *
 * Developers modifying this section of code should be familiar with the red
 * black tree design template. Code comments assume an understanding of the
 * principles involved.
 *
 * For further information see:
 * https://en.wikipedia.org/wiki/Red%E2%80%93black_tree
 *
 * @{
 */

#include <stdlib.h>
#include <stdint.h>
#include <assert.h>

#include "data.h"

 /** @cond FORWARD_DECLARATIONS */
typedef struct fiftyone_degrees_tree_node_t fiftyoneDegreesTreeNode;
typedef struct fiftyone_degrees_tree_root_t fiftyoneDegreesTreeRoot;
/** @endcond */

/** Node structure defining a single node in the tree. */
typedef struct fiftyone_degrees_tree_node_t {
	int64_t key; /**< Numeric key associated with the data value */
	fiftyoneDegreesTreeRoot *root; /**< The current root node of the tree */
	fiftyoneDegreesTreeNode *parent; /**< Parent node or NULL if root */
	fiftyoneDegreesTreeNode *left; /**< Left node or NULL if none */
	fiftyoneDegreesTreeNode *right; /**< Right node or NULL if none */
	unsigned char colour; /**< The colour of the node in the red black tree */
} fiftyoneDegreesTreeNode;

/** Tree root structure defining the beginning of the tree. */
typedef struct fiftyone_degrees_tree_root_t {
	fiftyoneDegreesTreeNode root; /**< The current root node of the tree */
	fiftyoneDegreesTreeNode empty; /**< Empty tree node */
} fiftyoneDegreesTreeRoot;

/**
 * Callback method called while iterating over a tree.
 * @param state pointer to any data needed by the method
 * @param node at the current point in the tree
 */
typedef void(*fiftyoneDegreesTreeIterateMethod)(
	void* state,
	fiftyoneDegreesTreeNode* node);

/**
 * Empty tree node.
 */
extern fiftyoneDegreesTreeNode *fiftyoneDegreesTreeEmpty;

/**
 * Used by assert statements to validate the number of entries in the cache for
 * debugging should any changes be made to the logic. Should not be compiled in
 * release builds.
 * @param root pointer to the node being counted.
 * @return the number of children plus 1 for this current node.
 */
uint32_t fiftyoneDegreesTreeCount(fiftyoneDegreesTreeRoot *root);

/**
 * Removes the node from the tree so that it can be used again to store
 * another result. The node will come from the last item in the cache's
 * linked list.
 * @param node pointer to be deleted.
 */
void fiftyoneDegreesTreeDelete(fiftyoneDegreesTreeNode *node);

/**
 * Inserts the node into the red black tree.
 * @param node pointer to the node being inserted.
 */
void fiftyoneDegreesTreeInsert(fiftyoneDegreesTreeNode *node);

/**
 * Returns the node that matches the key code provided.
 * @param root of the tree to search in
 * @param key to get the item for
 * @return the corresponding node if it exists in the tree, otherwise NULL.
 */
fiftyoneDegreesTreeNode* fiftyoneDegreesTreeFind(
	fiftyoneDegreesTreeRoot *root,
	int64_t key);

/**
 * Initialises a newly allocated node.
 * @param node to initialise
 * @param root of the tree to which the node belongs
 */
void fiftyoneDegreesTreeNodeInit(
	fiftyoneDegreesTreeNode *node,
	fiftyoneDegreesTreeRoot *root);

/**
 * Removes a node from the tree it belongs to.
 * @param node to remove
 */
void fiftyoneDegreesTreeNodeRemove(fiftyoneDegreesTreeNode *node);

/**
 * Initialises a newly allocated tree root to a clean state.
 * @param root to initialise
 */
void fiftyoneDegreesTreeRootInit(fiftyoneDegreesTreeRoot *root);

/**
 * Iterates over all the nodes in the tree starting at the root provided,
 * calling the callback method with each node in the tree.
 * @param root of the tree to iterate
 * @param state pointer to pass to the callback method
 * @param callback method to call with each node
 */
void fiftyoneDegreesTreeIterateNodes(
	fiftyoneDegreesTreeRoot *root,
	void *state,
	fiftyoneDegreesTreeIterateMethod callback);

/**
 * @}
 */

#endif