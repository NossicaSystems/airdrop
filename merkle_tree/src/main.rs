use wasm_bindgen::prelude::*;
use sha256::digest;

// Basic merkle tree implementation
// This will produce merkle trees like the following (note the real values would be hashed)
// Example 1 - input 1,2,3
//  1    2    3   3
//    12        33
//        1233
//
// Example 2 - input 1,2,3,4,5,6
// 1   2   3   4   5    6
//  12       34      56    56
//      1234           5656
//           12345656

#[wasm_bindgen]
pub fn create_hash_tree(nodes: Vec<String>) ->Option<MerkleTree> {
    let mut working_vec: Vec<String> = vec![];
    for node in nodes {
        working_vec.push(digest(node));
    }
    let mut working_node_total: usize = working_vec.len();
    let mut steps: Vec<u8> = Vec::new();

    if working_vec.len() % 2 == 1 {
        working_vec.push(working_vec[working_node_total - 1].clone());
        working_node_total += 1;
    }

    let initial_length = working_node_total;
    let mut startpoint = 0;
    let mut vec_to_add: Vec<String> = Vec::new();

    loop {
        // make sure tree is even
        if working_node_total % 2 == 1 {
            working_vec.push(working_vec.last().unwrap().clone());
        }   

        for index in (startpoint..working_vec.len()).step_by(2) {
            vec_to_add.push(digest(working_vec[index].clone() + &working_vec[index + 1]));
        }

        startpoint = working_vec.len();
        working_vec.append(&mut vec_to_add.clone());
        working_node_total = working_vec.len();

        if (vec_to_add.len()) / 2 == 1 {
            steps.push((vec_to_add.len() + 1).try_into().unwrap());
        } else {
            steps.push((vec_to_add.len()).try_into().unwrap());
        }

        if vec_to_add.len() == 1 {
            return Some(MerkleTree {
                length: initial_length as u8,
                hashroot: working_vec.last().unwrap().clone(),
                steps,
                hash_tree: working_vec.clone(),
            });
        }
        vec_to_add.clear();
    }
}

// Use this to get the node chain for a given value.
// Returns None if the value is not found.
pub fn get_hash_proof(test: String, merkle_tree: MerkleTree) -> Option<Vec<String>> {
    let local_tree = merkle_tree;

    let steps = &local_tree.steps;
    let mut end_point: usize = local_tree.length as usize;
    let nodes: &Vec<String> = &local_tree.hash_tree;
    let mut hunted: String = test;
    let mut startpoint: usize = 0;
    let mut step_number = 0;
    let mut proof: Vec<String> = Vec::new();
    let mut index = 0;
    while startpoint + index < end_point {
        if hunted == local_tree.hashroot {
            proof.push(hunted);
            return Some(proof);
        }

        if nodes[startpoint + index] == hunted {
            proof.push(hunted);
            if index % 2 == 1 {
                // it is on the right hand side
                hunted =
                    digest(nodes[startpoint + index - 1].clone() + &nodes[startpoint + index]);
            } else {
                // it is on the left hand side
                hunted =
                    digest(nodes[startpoint + index].clone() + &nodes[startpoint + index + 1]);
            }
            startpoint = end_point;
            end_point += steps[step_number] as usize;
            step_number += 1;
            index = 0;
            continue;
        }

        index += 1;
    }
    None
}

// Use this to compare the user's proof with our's
pub fn check_proof(test: &ClaimNFTParams) -> bool {
    let claimer = digest(
        test.node
            .0
            .iter()
            .map(|byte| format!("{:02X}", byte))
            .collect::<Vec<String>>()
            .concat(),
    );

    let master_proof: Vec<String> = get_hash_proof(claimer, ).unwrap();
    master_proof == test.proof
}

// Checks to see whether a given value is in the tree
// Generally used in testing
pub fn check_hash_value(&self, test_address: String) -> bool {
    if self.merkle_tree.is_none() {
        return false;
    };
    let tree = self.merkle_tree.as_ref().unwrap();

    let steps = &tree.steps;
    let mut end_point = tree.length as usize;
    let nodes = &tree.hash_tree;
    let mut hunted = test_address;
    let mut startpoint = 0;
    let mut step_number = 0;

    let mut index: usize = 0;
    while startpoint + index < end_point {
        if hunted.eq(&tree.hashroot) {
            return true;
        }

        if nodes[startpoint + index] == hunted {
            if index % 2 == 1 {
                // it is on the right hand side
                hunted =
                    digest(nodes[startpoint + index - 1].clone() + &nodes[startpoint + index]);
            } else {
                // it is on the left hand side
                hunted =
                    digest(nodes[startpoint + index].clone() + &nodes[startpoint + index + 1]);
            }
            startpoint = end_point;
            end_point += steps[step_number] as usize;
            step_number += 1;
            index = 0;
            continue;
        }

        index += 1;
    }
    false
}


fn main() {

}
